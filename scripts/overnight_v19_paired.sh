#!/usr/bin/env bash
# scripts/overnight_v19_paired.sh — v19 paired retrain: ReLU+BN vs GELU+LN
#
# Runs 6 training runs: 3 seeds × 2 architectures on identical FTMB v5 data.
# Each run produces a separate model directory with results.json and epochs.jsonl.
# Script continues past individual run failures and records which completed.
#
# MADR 0066 hard gate: a partial-seed architecture (fewer than 3 completed seeds)
# automatically fails gate condition 1. No makeup runs.
#
# Usage:
#   ./scripts/overnight_v19_paired.sh                  # Full pipeline
#   ./scripts/overnight_v19_paired.sh --skip-data       # Skip data prep
#   ./scripts/overnight_v19_paired.sh --dry-run         # Show config, don't train
#   ./scripts/overnight_v19_paired.sh --epochs N        # Override epoch count
#
# Output:
#   output/multibranch-training/v19-blend.ftmb          — Training data (FTMB v5)
#   models/sherlock-v19-relu-s{42,43,44}/               — ReLU+BN models
#   models/sherlock-v19-gelu-s{42,43,44}/               — GELU+LN models
#   results/overnight-v19-paired.log                    — Full log
#
# Spec: orbit/specs/2026-04-25-v19-paired-retrain/spec.yaml
# Decision: 0068 (revisit GELU+LN), 0066 (hard gate)
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v19-paired.log"

SKIP_DATA=false
DRY_RUN=false
EPOCHS=100
PATIENCE=15
BATCH_SIZE=32
SEEDS=(42 43 44)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-data)   SKIP_DATA=true; shift ;;
        --dry-run)     DRY_RUN=true; shift ;;
        --epochs)      EPOCHS="$2"; shift 2 ;;
        --help|-h)
            sed -n '2,/^set -/p' "$0" | grep '^#' | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# Tee all output to log
exec > >(tee -a "$LOG_FILE") 2>&1

# ── Configuration ──────────────────────────────────────────────────────
RELU_CONFIG="models/sherlock-v13-config.json"
GELU_CONFIG="models/sherlock-v19-gelu-config.json"
FTMB_FILE="output/multibranch-training/v19-blend.ftmb"
DISTILLED_FILE="output/distillation-v3/sherlock_distilled.csv.gz"
HF_DATASET="meridian-online/sherlock-annotated"

# Run definitions: name|config|seed|lr|wd
RUNS=(
    "sherlock-v19-relu-s42|$RELU_CONFIG|42|0.0001|0.0001"
    "sherlock-v19-relu-s43|$RELU_CONFIG|43|0.0001|0.0001"
    "sherlock-v19-relu-s44|$RELU_CONFIG|44|0.0001|0.0001"
    "sherlock-v19-gelu-s42|$GELU_CONFIG|42|0.0001|0.0001"
    "sherlock-v19-gelu-s43|$GELU_CONFIG|43|0.0001|0.0001"
    "sherlock-v19-gelu-s44|$GELU_CONFIG|44|0.0001|0.0001"
)

echo "================================================================"
echo " v19 Paired Retrain — ReLU+BN vs GELU+LN"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo ""
echo " Architectures:"
echo "   ReLU+BN: $RELU_CONFIG"
echo "   GELU+LN: $GELU_CONFIG"
echo ""
echo " Sweep: ${#SEEDS[@]} seeds × 2 architectures = ${#RUNS[@]} runs"
echo " Seeds: ${SEEDS[*]}"
echo " Epochs: $EPOCHS, patience: $PATIENCE, batch: $BATCH_SIZE"
echo " Data: $FTMB_FILE (FTMB v5, v4 corpus + container + datetime)"
echo ""
echo " Spec: orbit/specs/2026-04-25-v19-paired-retrain/"
echo " Gate: MADR 0066 (3-seed + net_label ≥+1 + per-domain ≤3)"
echo "================================================================"
echo ""

if [[ "$DRY_RUN" == "true" ]]; then
    echo "[DRY RUN] Would execute ${#RUNS[@]} training runs:"
    for run_def in "${RUNS[@]}"; do
        IFS='|' read -r name config seed lr wd <<< "$run_def"
        arch=$(echo "$name" | grep -q "gelu" && echo "GELU+LN" || echo "ReLU+BN")
        echo "  $name: $arch, seed=$seed, lr=$lr, wd=$wd"
        echo "    config: $config"
        echo "    output: models/$name/"
    done
    echo ""
    echo "[DRY RUN] Data: $FTMB_FILE"
    echo "[DRY RUN] Epochs: $EPOCHS, patience: $PATIENCE"
    exit 0
fi

PIPELINE_START=$(date +%s)

# ── Pre-flight checks ─────────────────────────────────────────────────

echo "[Pre-flight] Checking prerequisites..."

if [[ ! -f "$RELU_CONFIG" ]]; then
    echo "FAIL: ReLU config not found: $RELU_CONFIG"
    exit 1
fi
if [[ ! -f "$GELU_CONFIG" ]]; then
    echo "FAIL: GELU config not found: $GELU_CONFIG"
    exit 1
fi

# Verify both configs have correct dimensions
for cfg in "$RELU_CONFIG" "$GELU_CONFIG"; do
    if ! grep -q '"n_classes": 240' "$cfg"; then
        echo "FAIL: $cfg — n_classes must be 240"
        exit 1
    fi
    if ! grep -q '"valid_dim": 240' "$cfg"; then
        echo "FAIL: $cfg — valid_dim must be 240"
        exit 1
    fi
done

# Verify GELU config actually has GELU
if ! grep -q '"GELU"' "$GELU_CONFIG"; then
    echo "FAIL: GELU config missing activation: GELU"
    exit 1
fi
if ! grep -q '"use_layer_norm": true' "$GELU_CONFIG"; then
    echo "FAIL: GELU config missing use_layer_norm: true"
    exit 1
fi

echo "[Pre-flight] Configs: OK (ReLU+BN and GELU+LN verified)"

# Model2Vec + sibling-context
if [[ ! -d models/model2vec ]] || [[ ! -f models/model2vec/model.safetensors ]]; then
    echo "FAIL: Model2Vec not found at models/model2vec/"
    exit 1
fi
echo "[Pre-flight] Model2Vec: OK"

SIBLING_CTX_DIR="models/sibling-context"
if [[ ! -f "$SIBLING_CTX_DIR/model.safetensors" ]]; then
    echo "FAIL: Sibling-context model not found at $SIBLING_CTX_DIR/"
    exit 1
fi
echo "[Pre-flight] Sibling-context: OK"

# Build with Metal
echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"

# Verify feature extraction
echo "[Pre-flight] Verifying header feature extraction..."
HEADER_NONZERO=$(echo '["hello world", "test value", "example"]' | \
    ./target/release/finetype extract-features --json --header "city_name" 2>/dev/null | \
    python3 -c "
import json, sys
data = json.load(sys.stdin)
hf = data.get('header_features', [])
nonzero = sum(1 for x in hf if abs(x) > 1e-6)
print(nonzero)
" 2>/dev/null || echo "0")

if [[ "$HEADER_NONZERO" -lt 10 ]]; then
    echo "FAIL: Header features near-zero ($HEADER_NONZERO/128 nonzero)"
    exit 1
fi
echo "[Pre-flight] Header extraction OK ($HEADER_NONZERO/128 nonzero)"
echo ""

# ── Step 1: Prepare Training Data ─────────────────────────────────────

mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1: Data prep — SKIPPED (--skip-data)"
    echo "================================================================"
    if [[ -f scripts/read_ftmb.py ]]; then
        python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    fi
else
    # Download distilled data if needed
    if [[ ! -f "$DISTILLED_FILE" ]]; then
        echo "[Data] Distilled data not found. Downloading from HuggingFace..."
        mkdir -p output/distillation-v3
        if command -v uv >/dev/null 2>&1; then
            uv run --with datasets python3 -c "
from datasets import load_dataset
import csv, gzip
print('Downloading $HF_DATASET from HuggingFace...')
ds = load_dataset('$HF_DATASET', split='train')
print(f'Downloaded {len(ds)} rows')
with gzip.open('$DISTILLED_FILE', 'wt', newline='') as f:
    writer = csv.DictWriter(f, fieldnames=ds.column_names)
    writer.writeheader()
    for row in ds:
        writer.writerow(row)
print(f'Saved {len(ds)} rows to $DISTILLED_FILE')
" 2>&1
        else
            echo "FAIL: uv not found for HF download"
            exit 1
        fi
    fi

    [[ -f "$FTMB_FILE" ]] && rm -f "$FTMB_FILE"

    echo "================================================================"
    echo " Step 1: Prepare Training Data (FTMB v5)"
    echo " Started: $(date)"
    echo " Base: v4 corpus (v3 distilled + v4 UA/LOINC loaders)"
    echo " New: container TABLE_TEMPLATES + datetime generator improvements"
    echo "================================================================"

    python3 scripts/prepare_multibranch_data.py \
        --distilled "$DISTILLED_FILE" \
        --finetype ./target/release/finetype \
        --output "$FTMB_FILE" \
        --label-remap data/label_remap.json \
        --samples-per-type 1200 \
        --synthetic-columns 1200 \
        --ratio-distilled 0.7 \
        --augmentation-rate 0.35 \
        --filter-distilled \
        --decontaminate \
        --distilled-cap 600 \
        --hard-negatives 75 \
        --accounting-negatives 50 \
        --status-negatives 25 \
        --format v4 \
        --seed 42 \
        --workers 8

    echo ""
    echo "[Data] Verifying output..."
    if [[ -f scripts/read_ftmb.py ]]; then
        python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    fi
    echo "[Data] Complete: $(date)"
fi
echo ""

# ── Step 1.5: Pre-training data audit gate ────────────────────────────

echo "================================================================"
echo " Step 1.5: Pre-training Data Audit Gate (v19)"
echo "================================================================"

AUDIT_RESULT=$(python3 -c "
import struct, sys

path = '$FTMB_FILE'
with open(path, 'rb') as f:
    magic = f.read(4)
    assert magic == b'FTMB', f'Bad magic: {magic}'
    version = struct.unpack('<I', f.read(4))[0]
    n_records = struct.unpack('<Q', f.read(8))[0]
    char_dim = struct.unpack('<H', f.read(2))[0]
    embed_dim = struct.unpack('<H', f.read(2))[0]
    stats_dim = struct.unpack('<H', f.read(2))[0]
    header_dim = struct.unpack('<H', f.read(2))[0]

    if version < 4:
        print(f'FAIL: FTMB version {version}, expected v4+')
        sys.exit(1)

    n_groups = struct.unpack('<H', f.read(2))[0]
    _reserved = struct.unpack('<H', f.read(2))[0]
    valid_dim = struct.unpack('<H', f.read(2))[0]

    # Gate 1: VALID_DIM must be 240
    if valid_dim != 240:
        print(f'FAIL: valid_dim={valid_dim}, expected 240')
        sys.exit(1)
    print(f'Gate 1: valid_dim=240 — PASS')

    record_size = char_dim*4 + embed_dim*4 + stats_dim*4 + header_dim*4 + valid_dim*4

    label_counts = {}
    for g in range(n_groups):
        n_columns = struct.unpack('<H', f.read(2))[0]
        n_sibling_headers = struct.unpack('<H', f.read(2))[0]
        for _ in range(n_sibling_headers):
            name_len = struct.unpack('<H', f.read(2))[0]
            f.read(name_len)
        for c in range(n_columns):
            label_len = struct.unpack('<H', f.read(2))[0]
            label = f.read(label_len).decode('utf-8')
            _col_idx = struct.unpack('<H', f.read(2))[0]
            f.read(record_size)
            label_counts[label] = label_counts.get(label, 0) + 1

# Gate 2: No type below 50 columns
min_type_count = min(label_counts.values())
print(f'Gate 2: Min type count = {min_type_count} (gate: >=50)')
if min_type_count < 50:
    thin = {k: v for k, v in label_counts.items() if v < 50}
    print(f'FAIL: {len(thin)} types below 50:')
    for k, v in sorted(thin.items()):
        print(f'  {k}: {v}')
    sys.exit(1)
print(f'  PASS')

# Gate 3: Total types >= 240 (all taxonomy types must be present)
total_types = len(label_counts)
print(f'Gate 3: Total types = {total_types} (gate: >=240)')
if total_types < 240:
    print(f'FAIL: Only {total_types} types (need 240)')
    sys.exit(1)
print(f'  PASS')

# Gate 4: Max type count <= 1500
max_type_count = max(label_counts.values())
print(f'Gate 4: Max type count = {max_type_count} (gate: <=1500)')
if max_type_count > 1500:
    heavy = {k: v for k, v in label_counts.items() if v > 1500}
    print(f'WARN: {len(heavy)} types above 1500')

# Gate 5 (v19): v4-rehabilitated types PRESENT (v16 dropped these; v4 loaders restored them)
_rehabilitated = ['finance.banking.swift_bic', 'technology.internet.http_method',
                  'representation.file.excel_format', 'identity.medical.loinc',
                  'identity.medical.cpt', 'identity.government.ssn',
                  'technology.internet.user_agent']
missing_rehab = [t for t in _rehabilitated if t not in label_counts]
if missing_rehab:
    print(f'WARN: {len(missing_rehab)} v4-rehabilitated types missing: {missing_rehab}')
    # Not a hard fail — some may still be filtered by distilled-cap or decontamination
present_rehab = [t for t in _rehabilitated if t in label_counts]
print(f'Gate 5: {len(present_rehab)}/{len(_rehabilitated)} v4-rehabilitated types present — PASS')

# Gate 6 (v19): Container types present with sufficient volume
container_types = [k for k in label_counts if k.startswith('container.')]
container_min = min((label_counts[k] for k in container_types), default=0) if container_types else 0
print(f'Gate 6: Container types = {len(container_types)}, min count = {container_min} (gate: >=50)')
if container_min < 50:
    thin_c = {k: label_counts[k] for k in container_types if label_counts[k] < 50}
    if thin_c:
        print(f'FAIL: Container types below 50:')
        for k, v in sorted(thin_c.items()):
            print(f'  {k}: {v}')
        sys.exit(1)
print(f'  PASS')

print(f'')
print(f'Audit summary: {total_types} types, {sum(label_counts.values())} records')
print(f'  Per-type range: {min_type_count} — {max_type_count}')
print(f'ALL GATES PASSED')
" 2>&1)

echo "$AUDIT_RESULT"

if echo "$AUDIT_RESULT" | grep -q "^FAIL:"; then
    echo ""
    echo "Pre-training audit gate FAILED. Aborting."
    exit 1
fi
echo ""

# ── Step 2: Training Runs ─────────────────────────────────────────────

declare -A RUN_STATUS
declare -A RUN_TIME

echo "================================================================"
echo " Step 2: Training — ${#RUNS[@]} runs"
echo " Started: $(date)"
echo "================================================================"
echo ""

RUN_NUM=0
for run_def in "${RUNS[@]}"; do
    IFS='|' read -r name config seed lr wd <<< "$run_def"
    RUN_NUM=$((RUN_NUM + 1))
    MODEL_DIR="models/$name"
    ARCH=$(echo "$name" | grep -q "gelu" && echo "GELU+LN" || echo "ReLU+BN")

    echo "────────────────────────────────────────────────────────────"
    echo " Run $RUN_NUM/${#RUNS[@]}: $name ($ARCH)"
    echo " Config: $config"
    echo " Seed: $seed, LR: $lr, WD: $wd"
    echo " Started: $(date)"
    echo "────────────────────────────────────────────────────────────"

    if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
        echo "[Skip] Model already exists at $MODEL_DIR — skipping"
        RUN_STATUS[$name]="SKIPPED"
        RUN_TIME[$name]="0"
        continue
    fi

    RUN_START=$(date +%s)

    if cargo run --bin finetype --no-default-features --features metal --release -- \
        train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$MODEL_DIR" \
        --model-config "$config" \
        --epochs "$EPOCHS" \
        --batch-size "$BATCH_SIZE" \
        --lr "$lr" \
        --weight-decay "$wd" \
        --dropout 0.35 \
        --seed "$seed" \
        --head flat \
        --patience "$PATIENCE" \
        2>&1; then

        RUN_END=$(date +%s)
        RUN_ELAPSED=$(( (RUN_END - RUN_START) / 60 ))
        RUN_STATUS[$name]="OK"
        RUN_TIME[$name]="$RUN_ELAPSED"

        # Post-training: inject type_index_keys
        SAVED_CONFIG="$MODEL_DIR/config.json"
        if [[ -f "$SAVED_CONFIG" ]] && ! grep -q '"type_index_keys"' "$SAVED_CONFIG"; then
            echo "[Post-train] Injecting type_index_keys..."
            TYPE_KEYS=$(echo '["test"]' | \
                ./target/release/finetype extract-features --json --header "test" --validation 2>/dev/null | \
                python3 -c "import json, sys; print(json.dumps(json.load(sys.stdin)['type_index_keys']))" 2>/dev/null)
            if [[ -n "$TYPE_KEYS" ]] && [[ "$TYPE_KEYS" != "null" ]]; then
                python3 -c "
import json
with open('$SAVED_CONFIG') as f:
    config = json.load(f)
config['type_index_keys'] = json.loads('$TYPE_KEYS')
with open('$SAVED_CONFIG', 'w') as f:
    json.dump(config, f, indent=2)
    f.write('\n')
print(f'Injected {len(config[\"type_index_keys\"])} type_index_keys')
"
            fi
        fi

        echo ""
        echo "[Run $RUN_NUM] COMPLETED in ${RUN_ELAPSED} min"

        # Print training summary
        RESULTS_JSON="$MODEL_DIR/results.json"
        if [[ -f "$RESULTS_JSON" ]] && command -v duckdb >/dev/null 2>&1; then
            RESULTS_COLS="columns={epoch: 'INTEGER', train_loss: 'FLOAT', val_loss: 'FLOAT', train_accuracy: 'FLOAT', val_accuracy: 'FLOAT', learning_rate: 'DOUBLE', epoch_time_secs: 'FLOAT'}"
            duckdb -c "
                WITH flat AS (
                    SELECT * FROM read_json('$RESULTS_JSON', format='array', $RESULTS_COLS)
                ),
                best AS (
                    SELECT * FROM flat ORDER BY val_accuracy DESC LIMIT 1
                )
                SELECT
                    (SELECT count(*) FROM flat) AS total_epochs,
                    ROUND(best.val_accuracy * 100, 1) AS best_val_acc_pct,
                    best.epoch + 1 AS best_epoch,
                    ROUND(best.val_loss, 4) AS best_val_loss
                FROM best;
            " 2>/dev/null || true
        fi
    else
        RUN_END=$(date +%s)
        RUN_ELAPSED=$(( (RUN_END - RUN_START) / 60 ))
        RUN_STATUS[$name]="FAILED"
        RUN_TIME[$name]="$RUN_ELAPSED"
        echo ""
        echo "[Run $RUN_NUM] FAILED after ${RUN_ELAPSED} min — continuing to next run"
    fi
    echo ""
done

# ── Step 3: Summary ──────────────────────────────────────────────────

PIPELINE_END=$(date +%s)
TOTAL_ELAPSED=$(( (PIPELINE_END - PIPELINE_START) / 60 ))

echo "================================================================"
echo " v19 Paired Retrain — Summary"
echo " Finished: $(date)"
echo " Total elapsed: ${TOTAL_ELAPSED} minutes"
echo "================================================================"
echo ""

printf "%-30s %-10s %-10s\n" "Run" "Status" "Time (min)"
printf "%-30s %-10s %-10s\n" "---" "------" "----------"
for run_def in "${RUNS[@]}"; do
    IFS='|' read -r name _ _ _ _ <<< "$run_def"
    printf "%-30s %-10s %-10s\n" "$name" "${RUN_STATUS[$name]:-UNKNOWN}" "${RUN_TIME[$name]:-?}"
done
echo ""

# Check gate condition 1: all 3 seeds per architecture must complete
RELU_OK=0
GELU_OK=0
for seed in "${SEEDS[@]}"; do
    [[ "${RUN_STATUS[sherlock-v19-relu-s$seed]}" == "OK" || "${RUN_STATUS[sherlock-v19-relu-s$seed]}" == "SKIPPED" ]] && RELU_OK=$((RELU_OK + 1))
    [[ "${RUN_STATUS[sherlock-v19-gelu-s$seed]}" == "OK" || "${RUN_STATUS[sherlock-v19-gelu-s$seed]}" == "SKIPPED" ]] && GELU_OK=$((GELU_OK + 1))
done

echo "Gate condition 1 (3 seeds completed):"
echo "  ReLU+BN: $RELU_OK/3 $([ $RELU_OK -eq 3 ] && echo "✓" || echo "✗ FAIL")"
echo "  GELU+LN: $GELU_OK/3 $([ $GELU_OK -eq 3 ] && echo "✓" || echo "✗ FAIL")"
echo ""

if [[ $RELU_OK -lt 3 ]] && [[ $GELU_OK -lt 3 ]]; then
    echo "BOTH ARCHITECTURES FAILED gate condition 1. No eval to run."
    echo "Re-run the full sweep for the failed architecture(s)."
    exit 1
fi

echo "Next step: run scripts/v19_compare.sh for three-way comparison + gate evaluation"
echo ""
echo "Log: $LOG_FILE"
echo "================================================================"
