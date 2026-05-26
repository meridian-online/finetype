#!/usr/bin/env bash
# scripts/overnight_v23_precision.sh — v23 precision retrain
#
# Per spec 2026-05-27-v23-precision-retrain ac-02 + ac-03.
#
# Trains v23 on the v22 boundary blend + v23 hard negatives extracted
# from the multi-lens diagnostic's corpus pass. Additive over v22
# (Option A in the spec): v22's monotone-mover gains on geography
# are preserved by construction; the new signal is precision-
# targeted hard negatives for the top-6 corroborated false-positive
# clusters.
#
# Architecture is unchanged from v22 (5-branch, ReLU+BatchNorm, same
# seeds s42/s43/s44) so the data-composition delta is the isolated
# variable.
#
# Usage:
#   ./scripts/overnight_v23_precision.sh                  # Full pipeline
#   ./scripts/overnight_v23_precision.sh --skip-blend     # Reuse v23 blend
#   ./scripts/overnight_v23_precision.sh --skip-data      # Reuse FTMB
#   ./scripts/overnight_v23_precision.sh --dry-run        # Show config, don't train
#   ./scripts/overnight_v23_precision.sh --epochs N       # Override epoch count
#
# Output:
#   output/distillation-v23/                                          — blend
#   output/multibranch-training/v23-precision-blend.ftmb              — training data
#   models/sherlock-v23-precision-relu-s{42,43,44}/                   — trained models
#   results/overnight-v23-precision.log                                — full log
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v23-precision.log"

SKIP_BLEND=false
SKIP_DATA=false
DRY_RUN=false
EPOCHS=100
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
FTMB_FILE="output/multibranch-training/v23-precision-blend.ftmb"
V23_DISTILLED="output/distillation-v23/sherlock_distilled_v23.csv.gz"
V23_HARD_NEGATIVES="output/v23-precision-retrain/hard_negatives.parquet"
PYTHON_VENV="$PROJECT_DIR/eval/gittables/.venv/bin/python"
PYTHON=$([ -x "$PYTHON_VENV" ] && echo "$PYTHON_VENV" || echo "python3")

RUNS=(
    "sherlock-v23-precision-relu-s42|$RELU_CONFIG|42|0.0001|0.0001"
    "sherlock-v23-precision-relu-s43|$RELU_CONFIG|43|0.0001|0.0001"
    "sherlock-v23-precision-relu-s44|$RELU_CONFIG|44|0.0001|0.0001"
)

echo "================================================================"
echo " v23 Precision Retrain — v22 blend + corpus-pass hard negatives"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo ""
echo " Distilled:    $V23_DISTILLED"
echo " Hard negs:    $V23_HARD_NEGATIVES (114,301 cols across 6 clusters)"
echo " Architecture: ReLU+BN ($RELU_CONFIG)"
echo " Seeds:        ${SEEDS[*]}"
echo " Epochs:       $EPOCHS, patience: $PATIENCE, batch: $BATCH_SIZE"
echo ""
echo " Spec: .orbit/specs/2026-05-27-v23-precision-retrain/"
echo " ac-04 band: top-6 cluster cols drop ≥50% AND gated cell-2 ≤ -10.4%"
echo "             = Met; 20-50% drop within ±2pp = Partial; else Failed."
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

if [[ ! -f "$V23_HARD_NEGATIVES" ]]; then
    echo "FAIL: v23 hard negatives missing — run scripts/extract_v23_hard_negatives.py first"
    exit 1
fi
echo "[Pre-flight] v23 hard negatives: OK"

# Dataset integrity — verify all four registered training sources before
# committing to the multi-hour pipeline. (Same 4 sources as v22; v23
# adds hard-negative columns sourced from gittables/measure-half, which
# the existing leakage firewall covers — see spec 2026-05-27 ac-01.)
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

# ── Step 0: Build v23 blend (per ac-02) + audit gate ──────────────────
if [[ "$SKIP_BLEND" == "true" ]] && [[ -f "$V23_DISTILLED" ]]; then
    echo "================================================================"
    echo " Step 0: v23 blend — SKIPPED (--skip-blend)"
    echo "================================================================"
    BLEND_ROWS=$("$PYTHON" -c "import gzip; print(sum(1 for _ in gzip.open('$V23_DISTILLED', 'rt')) - 1)")
    echo "[Blend] Reusing $V23_DISTILLED ($BLEND_ROWS rows)"
else
    echo "================================================================"
    echo " Step 0: Build v23 distilled blend (with audit gate)"
    echo " Started: $(date)"
    echo "================================================================"
    if ! "$PYTHON" scripts/build_v23_distilled.py; then
        echo "FAIL: v23 blend audit gate failed — abort"
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
    echo " Step 1: Prepare Training Data (FTMB v5, v23 distilled)"
    echo " Started: $(date)"
    echo " Recipe: v22 boundary recipe verbatim, v23 distilled as input"
    echo "================================================================"

    # --distilled-cap raised from 600 → 1800 vs the v22 recipe.
    # Per-type record budget (--samples-per-type 1200) is unchanged so
    # training time is unchanged; the cap lift only widens the distilled
    # pool from which the blender draws 840 rows/type. For the two v23
    # target correct_labels (integer_number, discrete.categorical), the
    # combined v22+v23 pool exceeds 1800 → the blender picks ~840
    # distilled instead of being forced down to ~600 (and back-filled
    # with synthetic). Net effect: ~200 more hard-negative training rows
    # per target label without any training-time cost.
    python3 scripts/prepare_multibranch_data.py \
        --distilled "$V23_DISTILLED" \
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

# Gate 7a (v22 carry-over): geography retained.
geo_types = [k for k in label_counts if k.startswith('geography.')]
geo_total = sum(label_counts[k] for k in geo_types)
print(f'Gate 7a (v22 geography retained): Geography rows = {geo_total} (gate: >=2000)')
if geo_total < 2000:
    print(f'FAIL: geography rows {geo_total} < 2000 — v22 baseline lost in v23 blending')
    sys.exit(1)
print(f'  PASS — {geo_total} geography rows across {len(geo_types)} types')

# Gate 7c (v23 precision lift): the six hard-negative correct_labels
# must each be present with meaningful volume so the model has signal
# to learn the boundary. v22 already had thousands of these; v23 adds
# ~50k integer_number and ~50k categorical hard-neg rows on top.
v23_correct_labels = [
    'representation.discrete.categorical',
    'representation.numeric.integer_number',
]
for lbl in v23_correct_labels:
    n = label_counts.get(lbl, 0)
    print(f'Gate 7c ({lbl}): {n} rows (gate: >=600)')
    if n < 600:
        print(f'FAIL: {lbl} rows {n} < 600 — v23 hard-neg signal collapsed under cap')
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
echo "(models/default NOT swapped — symlink update is reviewer's call, see ac-05)"

# ── Step 4: Summary ──────────────────────────────────────────────────
PIPELINE_END=$(date +%s)
TOTAL_ELAPSED=$(( (PIPELINE_END - PIPELINE_START) / 60 ))

echo "================================================================"
echo " v23 Precision Retrain — Summary"
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
echo "       python3 scripts/gittables_corpus_pass.py --jobs 16 --execute --out-dir output/corpus-pass-v23  # ~7h"
echo "       python3 scripts/gittables_corpus_pass.py --fill-ydf --out-dir output/corpus-pass-v23          # ~25min"
echo "  3. Apply YDF validation gate against v23:"
echo "       python3 scripts/apply_ydf_validation_gate.py --columns output/corpus-pass-v23/corpus_pass/columns.parquet --out output/ydf-validation-gate/v23_gated.parquet"
echo "  4. Per-cluster FP rate (ac-04):"
echo "       python3 scripts/compute_v23_per_cluster_fp_rate.py"
echo "  5. Write cell_deltas_v23_vs_v22.md and read the ac-04 band:"
echo "       Met:     top-6 cluster cols drop >=50% AND gated cell-2 <= -10.4%"
echo "       Partial: top-6 drops 20-50% AND cell-2 within +/-2pp of v22"
echo "       Failed:  top-6 drops <20% OR cell-2 regresses >=2pp vs v22"
echo "  6. Take ac-05 Path A / B / C per band."
echo ""
echo "Log: $LOG_FILE"
echo "================================================================"
