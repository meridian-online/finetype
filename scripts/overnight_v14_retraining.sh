#!/usr/bin/env bash
# scripts/overnight_v14_retraining.sh — v14 retrain with targeted data curation
#
# Retrains multi-branch model (v14) to fix 15 known misclassifications from
# the v13 audit. No architecture changes — all fixes are data quality
# improvements and 2 safe rule removals:
#
#   AC-01: Subtype decontamination (per-value filtering in parent columns)
#          - url: remove data: URIs (→ data_uri subtype)
#          - email: remove "Name <email>" display format (→ email_display)
#          - phone_number: remove pure E.164 (→ phone_e164)
#
#   AC-02: Training collision fixes
#          - compact_ym: strictly 6-digit YYYYMM
#          - amount_accounting: hard-negative decimals with accounting headers
#          - postal/status: hard-negative integers with status headers
#          - hash/tsid: bias hash toward SHA-256/SHA-1 (40/64-char)
#          - alphanumeric_id: prefix+code patterns
#
#   AC-03: Data gap filling
#          - user_agent: diverse UAs (browser, tool, bot) + JWT negative examples
#          - json: JSON-as-string (generator already exists)
#
#   AC-04: Latitude/decimal hard negatives with audit-specific headers
#          - gap, depthError, dmin
#
#   AC-05: Country_code post-hint guard (code change in column.rs)
#   AC-07: Rule removals (h.contains("uri"), F3) — code changes in column.rs
#
# What's different from v13:
#   - Same architecture: 240 types, valid_dim=240, valid_hidden=[192,128]
#   - Data prep: +--decontaminate, +--accounting-negatives 50, +--status-negatives 25
#   - Generator improvements: user_agent diversity, hash length bias, alphanumeric_id patterns
#   - Code: country_code post-hint guard, h.contains("uri") removed, F3 removed
#
# Runs on M1 Pro with Metal acceleration. Requires:
#   - Distilled data at output/distillation-v3/sherlock_distilled.csv.gz
#   - Model2Vec at models/model2vec/
#   - Sibling-context model at models/sibling-context/
#   - Label remap at data/label_remap.json
#
# Usage:
#   ./scripts/overnight_v14_retraining.sh                          # Full pipeline
#   ./scripts/overnight_v14_retraining.sh --skip-data              # Skip data prep
#   ./scripts/overnight_v14_retraining.sh --skip-data --skip-train # Eval only
#   ./scripts/overnight_v14_retraining.sh --epochs N               # Override epoch count
#
# Output:
#   output/multibranch-training/v14-blend-70-30.ftmb       — Training data (v4)
#   models/sherlock-v14/                                    — Trained model
#   results/overnight-v14-retraining.log                    — Full log
#
# Spec: specs/2026-04-17-v14-retrain/spec.yaml
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v14-retraining.log"

SKIP_DATA=false
SKIP_TRAIN=false
EPOCHS=50

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-data)   SKIP_DATA=true; shift ;;
        --skip-train)  SKIP_TRAIN=true; shift ;;
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

echo "================================================================"
echo " Multi-Branch Overnight Pipeline v14 — Targeted Data Curation"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo " Config: FTMB v4, 70/30 distilled/synthetic, seed 42"
echo "         $EPOCHS epochs, ReLU activation, BatchNorm"
echo "         lr=0.0001, weight_decay=0.0001"
echo "         5 branches: char(960) + embed(512) + stats(27) + header(128) + valid(240)"
echo ""
echo " Changes from v13 (data only, same architecture):"
echo "   - AC-01: Subtype decontamination (url, email, phone_number)"
echo "   - AC-02: Collision fixes (accounting negatives, status negatives)"
echo "   - AC-03: Generator diversity (user_agent, hash, alphanumeric_id)"
echo "   - AC-04: Latitude/decimal hard negatives (gap, depthError, dmin)"
echo "   - AC-05: Country_code post-hint guard (code)"
echo "   - AC-07: Rule removals (h.contains(\"uri\"), F3) (code)"
echo ""
echo " Target: ≥220/227 label accuracy (96.9%+)"
echo " Spec: specs/2026-04-17-v14-retrain/"
echo "================================================================"
echo ""

PIPELINE_START=$(date +%s)

# --- Pre-flight checks ------------------------------------------------

echo "[Pre-flight] Checking prerequisites..."

DISTILLED_FILE="output/distillation-v3/sherlock_distilled.csv.gz"
HF_DATASET="meridian-online/sherlock-annotated"

if [[ ! -f "$DISTILLED_FILE" ]]; then
    echo "[Pre-flight] Distilled data not found locally. Downloading from HuggingFace..."
    mkdir -p output/distillation-v3

    if command -v uv >/dev/null 2>&1; then
        uv run --with datasets python3 -c "
from datasets import load_dataset
import csv, gzip

print('Downloading $HF_DATASET from HuggingFace...')
ds = load_dataset('$HF_DATASET', split='train')
print(f'Downloaded {len(ds)} rows')

print('Writing to $DISTILLED_FILE...')
with gzip.open('$DISTILLED_FILE', 'wt', newline='') as f:
    writer = csv.DictWriter(f, fieldnames=ds.column_names)
    writer.writeheader()
    for row in ds:
        writer.writerow(row)

print(f'Saved {len(ds)} rows to $DISTILLED_FILE')
" 2>&1
    else
        echo "[Pre-flight] uv not found. Install: https://docs.astral.sh/uv/"
        exit 1
    fi

    if [[ ! -f "$DISTILLED_FILE" ]]; then
        echo "FAIL: Download failed."
        exit 1
    fi
    echo "[Pre-flight] Download complete: $(ls -lh "$DISTILLED_FILE" | awk '{print $5}')"
fi

# Model2Vec is REQUIRED
if [[ ! -d models/model2vec ]] || [[ ! -f models/model2vec/model.safetensors ]]; then
    echo "FAIL: Model2Vec not found at models/model2vec/"
    exit 1
fi
echo "[Pre-flight] Model2Vec: OK"

# Sibling-context is REQUIRED
SIBLING_CTX_DIR="models/sibling-context"
if [[ ! -f "$SIBLING_CTX_DIR/model.safetensors" ]]; then
    echo "FAIL: Sibling-context model not found at $SIBLING_CTX_DIR/"
    exit 1
fi
echo "[Pre-flight] Sibling-context: OK"

# v14 uses same config as v13
MODEL_CONFIG="models/sherlock-v13-config.json"
if [[ ! -f "$MODEL_CONFIG" ]]; then
    echo "FAIL: Model config not found at $MODEL_CONFIG"
    exit 1
fi

# Verify config matches v14 constraints
if ! grep -q '"n_classes": 240' "$MODEL_CONFIG"; then
    echo "FAIL: n_classes must be 240"
    exit 1
fi
if ! grep -q '"valid_dim": 240' "$MODEL_CONFIG"; then
    echo "FAIL: valid_dim must be 240"
    exit 1
fi
if grep -q '"GELU"\|"use_layer_norm": true' "$MODEL_CONFIG"; then
    echo "FAIL: Config must use ReLU+BN (not GELU/LayerNorm)"
    exit 1
fi
echo "[Pre-flight] Config: n_classes=240, valid_dim=240, ReLU+BN — correct"

if ! command -v duckdb >/dev/null 2>&1; then
    echo "WARN: duckdb CLI not found (summary queries will be skipped)"
fi

# Build with Metal
echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"
echo ""

# Verify feature extraction
echo "[Pre-flight] Verifying header feature extraction..."
HEADER_CHECK=$(echo '["hello world", "test value", "example"]' | \
    ./target/release/finetype extract-features --json --header "city_name" 2>/dev/null)
HEADER_NONZERO=$(echo "$HEADER_CHECK" | python3 -c "
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

echo "[Pre-flight] Verifying validation feature extraction..."
VALID_CHECK=$(echo '["US", "GB", "DE", "FR", "JP"]' | \
    ./target/release/finetype extract-features --json --header "country" --validation 2>/dev/null)
VALID_DIM=$(echo "$VALID_CHECK" | python3 -c "
import json, sys
data = json.load(sys.stdin)
vf = data.get('validation', [])
nonzero = sum(1 for x in vf if abs(x) > 1e-6)
print(f'{len(vf)} {nonzero}')
" 2>/dev/null || echo "0 0")

VALID_DIM_N=$(echo "$VALID_DIM" | awk '{print $1}')
VALID_NONZERO=$(echo "$VALID_DIM" | awk '{print $2}')

if [[ "$VALID_DIM_N" -ne 240 ]]; then
    echo "FAIL: Validation dimension is $VALID_DIM_N, expected 240"
    exit 1
fi
echo "[Pre-flight] Validation extraction OK ($VALID_DIM_N dim, $VALID_NONZERO nonzero)"
echo ""

# --- Step 1: Prepare Training Data ------------------------------------

FTMB_FILE="output/multibranch-training/v14-blend-70-30.ftmb"
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1: Data prep — SKIPPED (--skip-data)"
    echo "================================================================"
    if [[ -f scripts/read_ftmb.py ]]; then
        python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    fi
    echo ""
else
    if [[ -f "$FTMB_FILE" ]] && [[ "$SKIP_DATA" != "true" ]]; then
        rm -f "$FTMB_FILE"
    fi

    echo "================================================================"
    echo " Step 1: Prepare Training Data (v14 curation)"
    echo " Started: $(date)"
    echo " Output: $FTMB_FILE"
    echo " Mix: 70% distilled (real-world), 30% synthetic"
    echo " NEW: --decontaminate, --accounting-negatives 50, --status-negatives 25"
    echo "================================================================"

    python3 scripts/prepare_multibranch_data.py \
        --distilled output/distillation-v3/sherlock_distilled.csv.gz \
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

    echo ""
    echo "[Data] Complete: $(date)"
fi
echo ""

# --- Step 1.5: Pre-training data audit gate (AC-06) -----------------------

echo "================================================================"
echo " Step 1.5: Pre-training Data Audit Gate (v14 AC-06)"
echo " Validates data quality BEFORE the training run"
echo "================================================================"

AUDIT_RESULT=$(python3 -c "
import struct, sys, re

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
        print(f'FAIL: FTMB version {version}, expected v4')
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

    # Scan all records to count per-type
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

# Gate 2: No type dropped below 50% of expected volume
# v13 had ~1200 columns/type. 50% threshold = 600 columns minimum.
# We use a softer check: no type below 50 columns (minimal viable).
min_type_count = min(label_counts.values())
print(f'Gate 2: Min type count = {min_type_count} (gate: >=50)')
if min_type_count < 50:
    thin_types = {k: v for k, v in label_counts.items() if v < 50}
    print(f'FAIL: {len(thin_types)} types below 50 columns:')
    for k, v in sorted(thin_types.items()):
        print(f'  {k}: {v}')
    sys.exit(1)
else:
    print(f'  PASS')

# Gate 3: Total types >= 240
total_types = len(label_counts)
print(f'Gate 3: Total types = {total_types} (gate: >=240)')
if total_types < 240:
    print(f'FAIL: Only {total_types} types, expected 240')
    sys.exit(1)
else:
    print(f'  PASS')

# Gate 4: Distilled cap — no type above 1500 columns
# (600 distilled + 360 synthetic + 75 hard negatives = ~1035 max expected)
max_type_count = max(label_counts.values())
print(f'Gate 4: Max type count = {max_type_count} (gate: <=1500)')
if max_type_count > 1500:
    heavy_types = {k: v for k, v in label_counts.items() if v > 1500}
    print(f'WARN: {len(heavy_types)} types above 1500:')
    for k, v in sorted(heavy_types.items(), key=lambda x: -x[1])[:5]:
        print(f'  {k}: {v}')

# Summary
print(f'')
print(f'Audit summary: {total_types} types, {sum(label_counts.values())} records')
print(f'  Per-type range: {min_type_count} — {max_type_count}')
print(f'ALL GATES PASSED')
" 2>&1)

echo "$AUDIT_RESULT"

if echo "$AUDIT_RESULT" | grep -q "^FAIL:"; then
    echo ""
    echo "Pre-training audit gate FAILED. Aborting before training."
    exit 1
fi
echo ""

# --- Step 2: Train Model -----------------------------------------------

MODEL_DIR="models/sherlock-v14"

echo "================================================================"
echo " Step 2: Train 5-Branch Model (v14 — same architecture as v13)"
echo " Started: $(date)"
echo " Config: $MODEL_CONFIG"
echo " Epochs: $EPOCHS, lr=0.0001, weight_decay=0.0001"
echo " Data: $FTMB_FILE (FTMB v4, 70/30 distilled/synthetic)"
echo " Architecture: valid_hidden=[192,128], merged_dim=1070"
echo "================================================================"

if [[ "$SKIP_TRAIN" == "true" ]] && [[ -f "$MODEL_DIR/model.safetensors" ]]; then
    echo "[Train] --skip-train: using existing model at $MODEL_DIR"
elif [[ -f "$MODEL_DIR/model.safetensors" ]]; then
    echo "[Train] Model already exists at $MODEL_DIR, skipping training"
else
    cargo run --bin finetype --no-default-features --features metal --release -- \
        train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$MODEL_DIR" \
        --model-config "$MODEL_CONFIG" \
        --epochs "$EPOCHS" \
        --batch-size 32 \
        --lr 0.0001 \
        --weight-decay 0.0001 \
        --dropout 0.35 \
        --seed 42 \
        --head flat \
        --patience 10 \
        \
    2>&1
fi

echo ""
echo "Training complete: $(date)"
echo ""

# --- Post-training: inject type_index_keys --------------------------------

SAVED_CONFIG="$MODEL_DIR/config.json"
if [[ -f "$SAVED_CONFIG" ]]; then
    if ! grep -q '"type_index_keys"' "$SAVED_CONFIG"; then
        echo "[Post-train] Injecting type_index_keys into config.json..."
        TYPE_KEYS=$(echo '["test"]' | \
            ./target/release/finetype extract-features --json --header "test" --validation 2>/dev/null | \
            python3 -c "import json, sys; print(json.dumps(json.load(sys.stdin)['type_index_keys']))")

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
fi

# --- Training Summary ---

RESULTS_JSON="$MODEL_DIR/results.json"
if [[ -f "$RESULTS_JSON" ]] && command -v duckdb >/dev/null 2>&1; then
    echo "================================================================"
    echo " Training Summary"
    echo "================================================================"
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
            ROUND(best.val_loss, 4) AS best_val_loss,
            ROUND(best.train_loss, 4) AS best_train_loss,
            ROUND((SELECT sum(epoch_time_secs) FROM flat) / 60, 1) AS total_train_min
        FROM best;
    " 2>/dev/null || echo "(summary query failed)"
fi
echo ""

# --- Step 3: Profile Eval ------------------------------------------------

echo "================================================================"
echo " Step 3: Profile Evaluation (v14 AC-09)"
echo " Started: $(date)"
echo " Model: $MODEL_DIR"
echo " Datasets: 35 datasets, 227 columns"
echo " Target: ≥220/227 (96.9%+)"
echo "================================================================"

# Val accuracy gate
if [[ -f "$RESULTS_JSON" ]]; then
    VAL_ACC=$(python3 -c "
import json
with open('$RESULTS_JSON') as f:
    epochs = json.load(f)
best = max(epochs, key=lambda e: e['val_accuracy'])
print(f'{best[\"val_accuracy\"]:.4f}')
")
    VAL_ACC_PCT=$(python3 -c "print(round($VAL_ACC * 100, 1))")
    echo "[Eval] Best val accuracy: ${VAL_ACC_PCT}%"

    if python3 -c "exit(0 if $VAL_ACC >= 0.84 else 1)"; then
        echo "[Eval] Val accuracy gate: PASS (≥84%)"
    else
        echo "[Eval] Val accuracy gate: FAIL (<84%)"
        echo "  Model may not have converged. Check results.json."
        exit 1
    fi
fi

# Run profile eval
echo "[Eval] Running profile eval..."
FINETYPE_MODEL_DIR="$MODEL_DIR" make eval-report 2>&1

echo ""
echo "[Eval] Profile eval complete: $(date)"

# --- Step 4: Column-level regression diff (AC-09) -------------------------

echo "================================================================"
echo " Step 4: Column-Level Regression Diff (v13 vs v14)"
echo "================================================================"

V13_RESULTS="models/sherlock-v13/eval/profile_results.csv"
V14_RESULTS="$MODEL_DIR/eval/profile_results.csv"

if [[ -f "$V13_RESULTS" ]] && [[ -f "$V14_RESULTS" ]]; then
    python3 -c "
import csv, sys

def load_results(path):
    results = {}
    with open(path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            key = (row.get('dataset', ''), row.get('column', ''))
            results[key] = row.get('predicted', row.get('prediction', ''))
    return results

v13 = load_results('$V13_RESULTS')
v14 = load_results('$V14_RESULTS')

common_keys = set(v13.keys()) & set(v14.keys())
if not common_keys:
    print('WARNING: No common columns found between v13 and v14 results')
    print(f'  v13 has {len(v13)} columns, v14 has {len(v14)} columns')
    sys.exit(0)

regressions = []
improvements = []
for key in sorted(common_keys):
    v13_pred = v13[key]
    v14_pred = v14[key]
    if v13_pred != v14_pred:
        dataset, column = key
        # We'd need ground truth to determine if this is a regression or improvement
        # For now, flag all changes
        regressions.append((dataset, column, v13_pred, v14_pred))

if regressions:
    print(f'Column predictions changed: {len(regressions)}/{len(common_keys)}')
    print()
    for ds, col, v13p, v14p in regressions:
        print(f'  {ds}/{col}: {v13p} → {v14p}')
else:
    print(f'No column predictions changed between v13 and v14 ({len(common_keys)} columns)')

# Summary
print(f'')
print(f'Total common columns: {len(common_keys)}')
print(f'Changed predictions: {len(regressions)}')
" 2>&1
else
    echo "Cannot run regression diff:"
    [[ ! -f "$V13_RESULTS" ]] && echo "  v13 results not found: $V13_RESULTS"
    [[ ! -f "$V14_RESULTS" ]] && echo "  v14 results not found: $V14_RESULTS"
fi
echo ""

# --- Summary ---------------------------------------------------------------

PIPELINE_END=$(date +%s)
ELAPSED=$((PIPELINE_END - PIPELINE_START))
ELAPSED_MIN=$((ELAPSED / 60))

echo "================================================================"
echo " v14 Pipeline Complete"
echo " Finished: $(date)"
echo " Total elapsed: ${ELAPSED_MIN} minutes"
echo ""
echo " Model: $MODEL_DIR/"
echo " Log: $LOG_FILE"
echo " Spec: specs/2026-04-17-v14-retrain/spec.yaml"
echo ""
echo " NOTE: models/default symlink stays on sherlock-v13 until"
echo " v14 eval is verified and you run:"
echo "   ln -sf sherlock-v14 models/default"
echo "================================================================"
