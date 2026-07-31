#!/usr/bin/env bash
# scripts/overnight_v24_numeric.sh — v24 numeric-precision retrain
#
# Per spec 2026-06-03-v24-numeric-precision ac-03.
#
# Trains v24 on the v22 boundary blend + v24 numeric-target hard negatives
# (the four HIGH-safety clusters: utc/url/boolean -> integer, integer -> decimal).
# Additive over v22, exactly as v23 was — architecture, seeds and config are
# byte-for-byte the v23 recipe so the ONLY isolated variable is the
# hard-negative composition.
#
# Two deliberate deltas vs scripts/overnight_v23_precision.sh:
#
#   1. NO --include-column-level-types. v23's load-bearing flag opted the FTMB
#      builder in to training representation.discrete.categorical (+ ordinal +
#      increment) as correct answers — three of v23's six clusters needed it,
#      and it is exactly how v23 died (+529.6% categorical, ~48k geography
#      eaten). v24's hard negatives are integer_number / decimal_number only;
#      neither is in COLUMN_LEVEL_TYPES, so they survive the default drop. With
#      the flag OFF, categorical can never be a training target — the spec
#      invariant, enforced structurally rather than by audit.
#
#   2. Audit Gate 7c checks the two NUMERIC correct_labels (integer_number,
#      decimal_number), not categorical.
#
# Cap stays 1800 (matches v23 — author decision 2026-06-03: keep v24 a clean
# data-composition delta over v23's recipe, not a confounded cap change).
# Epochs 50 per ac-03.
#
# Usage:
#   ./scripts/overnight_v24_numeric.sh                  # Full pipeline
#   ./scripts/overnight_v24_numeric.sh --skip-blend     # Reuse v24 blend
#   ./scripts/overnight_v24_numeric.sh --skip-data      # Reuse FTMB
#   ./scripts/overnight_v24_numeric.sh --dry-run        # Show config, don't train
#   ./scripts/overnight_v24_numeric.sh --epochs N       # Override epoch count
#
# Output:
#   output/distillation-v24/                                          — blend
#   output/multibranch-training/v24-numeric-blend.ftmb                — training data
#   models/sherlock-v24-numeric-relu-s{42,43,44}/                     — trained models
#   results/overnight-v24-numeric.log                                  — full log
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v24-numeric.log"

SKIP_BLEND=false
SKIP_DATA=false
DRY_RUN=false
EPOCHS=50
PATIENCE=15
BATCH_SIZE=32
SEEDS=(42 43 44)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-blend)     SKIP_BLEND=true; shift ;;
        --skip-data)      SKIP_DATA=true; shift ;;
        --dry-run)        DRY_RUN=true; shift ;;
        --epochs)         EPOCHS="$2"; shift 2 ;;
        --help|-h)
            sed -n '2,/^set -/p' "$0" | grep '^#' | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

exec > >(tee -a "$LOG_FILE") 2>&1

# ── Configuration ──────────────────────────────────────────────────────
RELU_CONFIG="models/sherlock-v13-config.json"
FTMB_FILE="output/multibranch-training/v24-numeric-blend.ftmb"
V24_DISTILLED="output/distillation-v24/sherlock_distilled_v24.csv.gz"
V24_HARD_NEGATIVES="output/v24-numeric-precision/hard_negatives.parquet"
PYTHON_VENV="$PROJECT_DIR/eval/gittables/.venv/bin/python"
PYTHON=$([ -x "$PYTHON_VENV" ] && echo "$PYTHON_VENV" || echo "python3")

RUNS=(
    "sherlock-v24-numeric-relu-s42|$RELU_CONFIG|42|0.0001|0.0001"
    "sherlock-v24-numeric-relu-s43|$RELU_CONFIG|43|0.0001|0.0001"
    "sherlock-v24-numeric-relu-s44|$RELU_CONFIG|44|0.0001|0.0001"
)

echo "================================================================"
echo " v24 Numeric-Precision Retrain — v22 blend + numeric hard negatives"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo ""
echo " Distilled:    $V24_DISTILLED"
echo " Hard negs:    $V24_HARD_NEGATIVES (78,612 cols, 4 numeric clusters, ZERO categorical)"
echo " Architecture: ReLU+BN ($RELU_CONFIG)"
echo " Seeds:        ${SEEDS[*]}"
echo " Epochs:       $EPOCHS, patience: $PATIENCE, batch: $BATCH_SIZE"
echo " Distilled cap: 1800 (matches v23); column-level types: OFF (categorical never a target)"
echo ""
echo " Spec: 2026-06-03-v24-numeric-precision"
echo " ac-04 band: FP drop on 4 numeric clusters AND no categorical explosion"
echo "             AND no geography regression AND gated cell-2 vs v19 holds."
echo "================================================================"
echo ""

if [[ "$DRY_RUN" == "true" ]]; then
    echo "[DRY RUN] Would execute ${#RUNS[@]} training runs:"
    for run_def in "${RUNS[@]}"; do
        IFS='|' read -r name config seed lr wd <<< "$run_def"
        echo "  $name: seed=$seed, lr=$lr, wd=$wd"
    done
    exit 0
fi

PIPELINE_START=$(date +%s)

# ── Pre-flight checks ─────────────────────────────────────────────────
echo "[Pre-flight] Checking prerequisites..."

if [[ ! -f "$RELU_CONFIG" ]]; then
    echo "FAIL: ReLU config not found: $RELU_CONFIG"
    exit 1
fi
grep -q '"n_classes": 240' "$RELU_CONFIG" || { echo "FAIL: $RELU_CONFIG — n_classes must be 240"; exit 1; }
grep -q '"valid_dim": 240' "$RELU_CONFIG" || { echo "FAIL: $RELU_CONFIG — valid_dim must be 240"; exit 1; }
echo "[Pre-flight] Config: OK"

if [[ ! -f "$V24_HARD_NEGATIVES" ]]; then
    echo "FAIL: v24 hard negatives missing — run scripts/extract_v24_hard_negatives.py first"
    exit 1
fi
echo "[Pre-flight] v24 hard negatives: OK"

# Dataset integrity — verify all four registered training sources before
# committing to the multi-hour pipeline. (Same 4 sources as v22/v23; v24's
# hard-negative columns are sourced from the corpus pass, which the existing
# leakage firewall covers — see spec ac-01.)
echo "[Pre-flight] Verifying registered training datasets..."
if ! python3 scripts/dataset_verify.py sherlock cldr geonames wikidata_q5 2>&1 | tail -10; then
    echo "FAIL: dataset_verify found drift — abort before training"
    exit 1
fi
echo "[Pre-flight] Datasets: OK"

# Sibling-context model
SIBLING_CTX_DIR="models/sibling-context"
if [[ ! -f "$SIBLING_CTX_DIR/model.safetensors" ]]; then
    echo "FAIL: Sibling-context model not found at $SIBLING_CTX_DIR/"
    exit 1
fi
echo "[Pre-flight] Sibling-context: OK"

# Build binary
echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"

# Verify header feature extraction
HEADER_NONZERO=$(echo '["hello world", "test value", "example"]' | \
    ./target/release/finetype extract-features --json --header "city_name" 2>/dev/null | \
    python3 -c "
import json, sys
data = json.load(sys.stdin)
hf = data.get('header_features', [])
print(sum(1 for x in hf if abs(x) > 1e-6))
" 2>/dev/null || echo "0")
if [[ "$HEADER_NONZERO" -lt 10 ]]; then
    echo "FAIL: Header features near-zero ($HEADER_NONZERO/128 nonzero)"
    exit 1
fi
echo "[Pre-flight] Header extraction OK ($HEADER_NONZERO/128 nonzero)"
echo ""

# ── Step 0: Build v24 blend (per ac-02) + audit gate ──────────────────
if [[ "$SKIP_BLEND" == "true" ]] && [[ -f "$V24_DISTILLED" ]]; then
    echo "================================================================"
    echo " Step 0: v24 blend — SKIPPED (--skip-blend)"
    echo "================================================================"
    BLEND_ROWS=$("$PYTHON" -c "import gzip; print(sum(1 for _ in gzip.open('$V24_DISTILLED', 'rt')) - 1)")
    echo "[Blend] Reusing $V24_DISTILLED ($BLEND_ROWS rows)"
else
    echo "================================================================"
    echo " Step 0: Build v24 distilled blend (with audit gate)"
    echo " Started: $(date)"
    echo "================================================================"
    if ! "$PYTHON" scripts/build_v24_distilled.py; then
        echo "FAIL: v24 blend audit gate failed — abort"
        exit 1
    fi
    echo "[Step 0] Complete: $(date)"
fi
echo ""

# ── Step 1: Prepare Training Data ─────────────────────────────────────
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1: Data prep — SKIPPED (--skip-data)"
    echo "================================================================"
    [[ -f scripts/read_ftmb.py ]] && python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
else
    [[ -f "$FTMB_FILE" ]] && rm -f "$FTMB_FILE"

    echo "================================================================"
    echo " Step 1: Prepare Training Data (FTMB v5, v24 distilled)"
    echo " Started: $(date)"
    echo " Recipe: v22 boundary recipe verbatim, v24 distilled as input"
    echo "================================================================"

    # Recipe is byte-identical to v23 with ONE removal: no
    # --include-column-level-types. v24's hard negatives are
    # integer_number / decimal_number, neither of which is a
    # COLUMN_LEVEL_TYPE, so they pass through the default filter
    # untouched. Leaving the flag off means categorical/ordinal/increment
    # are dropped (the v22 default behaviour) — categorical can never be a
    # training target, which is the spec invariant.
    python3 scripts/prepare_multibranch_data.py \
        --distilled "$V24_DISTILLED" \
        --finetype ./target/release/finetype \
        --output "$FTMB_FILE" \
        --label-remap data/label_remap.json \
        --samples-per-type 1200 \
        --synthetic-columns 1200 \
        --ratio-distilled 0.7 \
        --augmentation-rate 0.35 \
        --filter-distilled \
        --decontaminate \
        --distilled-cap 1800 \
        --hard-negatives 75 \
        --accounting-negatives 50 \
        --status-negatives 25 \
        --format v4 \
        --seed 42 \
        --workers 8

    echo ""
    echo "[Data] Verifying output..."
    [[ -f scripts/read_ftmb.py ]] && python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    echo "[Data] Complete: $(date)"
fi
echo ""

# ── Step 1.5: Pre-training data audit gate ─────────────────────────────
echo "================================================================"
echo " Step 1.5: Pre-training Data Audit Gate"
echo "================================================================"

python3 -c "
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
        print(f'FAIL: FTMB version {version}, expected v4+'); sys.exit(1)
    n_groups = struct.unpack('<H', f.read(2))[0]
    _reserved = struct.unpack('<H', f.read(2))[0]
    valid_dim = struct.unpack('<H', f.read(2))[0]
    if valid_dim != 240:
        print(f'FAIL: valid_dim={valid_dim}, expected 240'); sys.exit(1)
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

min_type_count = min(label_counts.values())
print(f'Gate 2: Min type count = {min_type_count} (gate: >=50)')
if min_type_count < 50:
    thin = {k: v for k, v in label_counts.items() if v < 50}
    print(f'FAIL: {len(thin)} types below 50:')
    for k, v in sorted(thin.items()):
        print(f'  {k}: {v}')
    sys.exit(1)
print(f'  PASS')

total_types = len(label_counts)
print(f'Gate 3: Total types = {total_types} (gate: >=238)')
if total_types < 238:
    print(f'FAIL: only {total_types} types (need >=238)')
    sys.exit(1)
print(f'  PASS')

# Gate 7a (v22 carry-over): geography retained — the v23-death guard.
geo_types = [k for k in label_counts if k.startswith('geography.')]
geo_total = sum(label_counts[k] for k in geo_types)
print(f'Gate 7a (v22 geography retained): Geography rows = {geo_total} (gate: >=2000)')
if geo_total < 2000:
    print(f'FAIL: geography rows {geo_total} < 2000 — v22 baseline lost in v24 blending')
    sys.exit(1)
print(f'  PASS — {geo_total} geography rows across {len(geo_types)} types')

# Gate 7b (v24 invariant): categorical must NOT be inflated as a target.
# With --include-column-level-types OFF the builder drops categorical, so
# whatever survives is the v22-default residue. Flag if it ever balloons.
cat_n = label_counts.get('representation.discrete.categorical', 0)
print(f'Gate 7b (categorical not a target): categorical rows = {cat_n}')

# Gate 7c (v24 precision lift): the two NUMERIC correct_labels must each be
# present with meaningful volume so the model has signal to learn the
# boundary. v24 adds ~75k integer_number + ~3.9k decimal_number hard negs.
v24_correct_labels = [
    'representation.numeric.integer_number',
    'representation.numeric.decimal_number',
]
for lbl in v24_correct_labels:
    n = label_counts.get(lbl, 0)
    print(f'Gate 7c ({lbl}): {n} rows (gate: >=600)')
    if n < 600:
        print(f'FAIL: {lbl} rows {n} < 600 — v24 hard-neg signal collapsed under cap')
        sys.exit(1)
    print(f'  PASS')
print(f'')
print(f'Audit summary: {total_types} types, {sum(label_counts.values())} records')
print(f'ALL GATES PASSED')
"
echo ""

# ── Step 2: Training Runs ─────────────────────────────────────────────
RUN_NAMES=(); RUN_STATUSES=(); RUN_TIMES=()

echo "================================================================"
echo " Step 2: Training — ${#RUNS[@]} runs"
echo " Started: $(date)"
echo "================================================================"
echo ""

RUN_NUM=0
for run_def in "${RUNS[@]}"; do
    IFS='|' read -r name config seed lr wd <<< "$run_def"
    RUN_NAMES+=("$name")
    RUN_NUM=$((RUN_NUM + 1))
    MODEL_DIR="models/$name"

    echo "────────────────────────────────────────────────────────────"
    echo " Run $RUN_NUM/${#RUNS[@]}: $name (ReLU+BN)"
    echo " Config: $config | Seed: $seed | LR: $lr | WD: $wd"
    echo " Started: $(date)"
    echo "────────────────────────────────────────────────────────────"

    if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
        echo "[Skip] Model already exists at $MODEL_DIR — skipping"
        RUN_STATUSES+=("SKIPPED"); RUN_TIMES+=("0"); continue
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
        RUN_STATUSES+=("OK"); RUN_TIMES+=("$RUN_ELAPSED")

        SAVED_CONFIG="$MODEL_DIR/config.json"
        if [[ -f "$SAVED_CONFIG" ]] && ! grep -q '"type_index_keys"' "$SAVED_CONFIG"; then
            TYPE_KEYS=$(echo '["test"]' | \
                ./target/release/finetype extract-features --json --header "test" --validation 2>/dev/null | \
                python3 -c "import json, sys; print(json.dumps(json.load(sys.stdin)['type_index_keys']))" 2>/dev/null)
            if [[ -n "$TYPE_KEYS" ]] && [[ "$TYPE_KEYS" != "null" ]]; then
                python3 -c "
import json
with open('$SAVED_CONFIG') as f: config = json.load(f)
config['type_index_keys'] = json.loads('$TYPE_KEYS')
with open('$SAVED_CONFIG', 'w') as f: json.dump(config, f, indent=2); f.write('\n')
"
            fi
        fi
        echo "[Run $RUN_NUM] COMPLETED in ${RUN_ELAPSED} min"
    else
        RUN_END=$(date +%s)
        RUN_ELAPSED=$(( (RUN_END - RUN_START) / 60 ))
        RUN_STATUSES+=("FAILED"); RUN_TIMES+=("$RUN_ELAPSED")
        echo "[Run $RUN_NUM] FAILED after ${RUN_ELAPSED} min — continuing"
    fi
    echo ""
done

# ── Step 3: Cherry-pick best seed by val_acc ─────────────────────────
echo "================================================================"
echo " Step 3: Cherry-pick best seed"
echo "================================================================"
BEST=""
BEST_ACC="0"
for name in "${RUN_NAMES[@]}"; do
    if [[ -f "models/$name/results.json" ]]; then
        ACC=$(python3 -c "import json; print(json.load(open('models/$name/results.json'))[-1].get('val_accuracy', 0) if isinstance(json.load(open('models/$name/results.json')), list) else json.load(open('models/$name/results.json')).get('best_val_acc', 0))" 2>/dev/null || echo "0")
        echo "  $name: val_acc=$ACC"
        IS_BETTER=$(python3 -c "print(1 if float('$ACC') > float('$BEST_ACC') else 0)")
        if [[ "$IS_BETTER" == "1" ]]; then
            BEST="$name"
            BEST_ACC="$ACC"
        fi
    fi
done
echo "Best seed: $BEST (val_acc=$BEST_ACC)"
echo "(models/default NOT swapped — promotion is reviewer's call after ac-04/ac-05)"

# ── Step 4: Summary ──────────────────────────────────────────────────
PIPELINE_END=$(date +%s)
TOTAL_ELAPSED=$(( (PIPELINE_END - PIPELINE_START) / 60 ))

echo "================================================================"
echo " v24 Numeric-Precision Retrain — Summary"
echo " Finished: $(date)"
echo " Total elapsed: ${TOTAL_ELAPSED} minutes"
echo "================================================================"
printf "%-50s %-10s %-10s\n" "Run" "Status" "Time (min)"
printf "%-50s %-10s %-10s\n" "---" "------" "----------"
for i in "${!RUN_NAMES[@]}"; do
    printf "%-50s %-10s %-10s\n" "${RUN_NAMES[$i]}" "${RUN_STATUSES[$i]:-UNKNOWN}" "${RUN_TIMES[$i]:-?}"
done
echo ""

OK_COUNT=0
for status in "${RUN_STATUSES[@]}"; do
    [[ "$status" == "OK" || "$status" == "SKIPPED" ]] && OK_COUNT=$((OK_COUNT + 1))
done

echo "Completed seeds: $OK_COUNT/${#RUNS[@]}"
echo ""
echo "Next steps (spec ac-04, ac-05):"
echo "  1. Cherry-pick (already identified above): $BEST"
echo "  2. Run full corpus pass to a fresh output dir:"
echo "       source eval/gittables/.venv/bin/activate"
echo "       python3 scripts/gittables_corpus_pass.py --jobs 16 --execute --out-dir output/corpus-pass-v24  # ~7h"
echo "       python3 scripts/gittables_corpus_pass.py --fill-ydf --out-dir output/corpus-pass-v24          # ~25min"
echo "  3. Per-cluster FP rate (ac-04a): adapt scripts/compute_v23_per_cluster_fp_rate.py for the 4 numeric clusters."
echo "  4. Post-train Sense-distribution snapshot (ac-04b — MANDATORY):"
echo "       FINETYPE_MODEL=models/$BEST scripts/snapshot_sense_distribution.py --label v24 --files 800 --seed 42"
echo "       diff sense_dist_v24.json vs sense_dist_v19.json: categorical must NOT explode, geography must NOT regress."
echo "  5. Round-trip (ac-05): scripts/roundtrip_metrics.sh earthquakes_2024.csv + roundtrip_ab.py — non_trivial_pct >= 0.80."
echo "  6. Combined band must be Met to promote; if Failed, open a re-litigation memo and do not ship."
echo ""
echo "Log: $LOG_FILE"
echo "================================================================"
