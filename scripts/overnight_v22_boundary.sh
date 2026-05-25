#!/usr/bin/env bash
# scripts/overnight_v22_boundary.sh — v22 boundary-training retrain
#
# Per spec 2026-05-25-v22-boundary-training ac-05.
#
# Trains v22 on the boundary-aware blend: v19 distilled + v21 GeoNames
# geography + ac-03 corroborated_gaps positive labels + ac-02 mined
# v21 hard negatives + ac-02 wikidata-derived person-name columns. The
# v21 retrain shape (3 ReLU seeds @ v13 config) is preserved so the
# data-composition delta is the isolated variable.
#
# Usage:
#   ./scripts/overnight_v22_boundary.sh                  # Full pipeline
#   ./scripts/overnight_v22_boundary.sh --skip-mining    # Reuse ac-01/02/03 outputs
#   ./scripts/overnight_v22_boundary.sh --skip-blend     # Reuse v22 blend
#   ./scripts/overnight_v22_boundary.sh --skip-data      # Reuse FTMB
#   ./scripts/overnight_v22_boundary.sh --dry-run        # Show config, don't train
#   ./scripts/overnight_v22_boundary.sh --epochs N       # Override epoch count
#
# Output:
#   output/distillation-v22/                                          — mining + blend
#   output/multibranch-training/v22-boundary-blend.ftmb              — training data
#   models/sherlock-v22-boundary-relu-s{42,43,44}/                   — trained models
#   results/overnight-v22-boundary.log                                — full log
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v22-boundary.log"

SKIP_MINING=false
SKIP_BLEND=false
SKIP_DATA=false
DRY_RUN=false
EPOCHS=100
PATIENCE=15
BATCH_SIZE=32
SEEDS=(42 43 44)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-mining)    SKIP_MINING=true; shift ;;
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
FTMB_FILE="output/multibranch-training/v22-boundary-blend.ftmb"
V22_DISTILLED="output/distillation-v22/sherlock_distilled_v22.csv.gz"
PYTHON_VENV="$PROJECT_DIR/eval/gittables/.venv/bin/python"
PYTHON=$([ -x "$PYTHON_VENV" ] && echo "$PYTHON_VENV" || echo "python3")

RUNS=(
    "sherlock-v22-boundary-relu-s42|$RELU_CONFIG|42|0.0001|0.0001"
    "sherlock-v22-boundary-relu-s43|$RELU_CONFIG|43|0.0001|0.0001"
    "sherlock-v22-boundary-relu-s44|$RELU_CONFIG|44|0.0001|0.0001"
)

echo "================================================================"
echo " v22 Boundary Retrain — v21 blend + ac-02 hard negatives + ac-03 corroborated + wikidata persons"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo ""
echo " Distilled:    $V22_DISTILLED"
echo " Architecture: ReLU+BN ($RELU_CONFIG)"
echo " Seeds:        ${SEEDS[*]}"
echo " Epochs:       $EPOCHS, patience: $PATIENCE, batch: $BATCH_SIZE"
echo ""
echo " Spec: .orbit/specs/2026-05-25-v22-boundary-training/"
echo " ac-08: cell-2 lift ≥20% — closed downstream by corpus pass."
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

# Dataset integrity — verify all four registered training sources before
# committing to the multi-hour pipeline.
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

# ── Step -1: Regenerate v22 distillation sources (if needed) ──────────
need_regenerate=false
if [[ ! -f "output/distillation-v22/corroborated_gaps_distilled.csv.gz" ]]; then need_regenerate=true; fi
if [[ ! -f "output/distillation-v22/hard_negatives_mined.csv.gz" ]]; then need_regenerate=true; fi
if [[ ! -f "output/distillation-v22/wikidata_persons.csv.gz" ]]; then need_regenerate=true; fi

if [[ "$SKIP_MINING" == "true" ]] && [[ "$need_regenerate" == "false" ]]; then
    echo "================================================================"
    echo " Step -1: v22 mining — SKIPPED (--skip-mining + outputs cached)"
    echo "================================================================"
else
    echo "================================================================"
    echo " Step -1: Regenerate v22 distillation sources"
    echo " Started: $(date)"
    echo "================================================================"

    echo "[Step -1a] Mining corroborated_gaps (ac-03)..."
    "$PYTHON" scripts/mine_corroborated_gaps_training.py 2>&1 | tail -8

    echo "[Step -1b] Mining v21 hard negatives (ac-02)..."
    "$PYTHON" scripts/mine_v21_hard_negatives.py 2>&1 | tail -8

    echo "[Step -1c] Generating Wikidata person columns (ac-02)..."
    "$PYTHON" scripts/generate_wikidata_person_columns.py 2>&1 | tail -8

    echo "[Step -1] Complete: $(date)"
fi
echo ""

# ── Step 0: Build v22 blend (per ac-04) + audit gate ──────────────────
if [[ "$SKIP_BLEND" == "true" ]] && [[ -f "$V22_DISTILLED" ]]; then
    echo "================================================================"
    echo " Step 0: v22 blend — SKIPPED (--skip-blend)"
    echo "================================================================"
    BLEND_ROWS=$("$PYTHON" -c "import gzip; print(sum(1 for _ in gzip.open('$V22_DISTILLED', 'rt')) - 1)")
    echo "[Blend] Reusing $V22_DISTILLED ($BLEND_ROWS rows)"
else
    echo "================================================================"
    echo " Step 0: Build v22 distilled blend (with audit gate)"
    echo " Started: $(date)"
    echo "================================================================"
    if ! "$PYTHON" scripts/build_v22_distilled.py; then
        echo "FAIL: v22 blend audit gate failed — abort"
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
    echo " Step 1: Prepare Training Data (FTMB v5, v22 distilled)"
    echo " Started: $(date)"
    echo " Base: v19 recipe (filter_distilled + decontaminate)"
    echo " Cap:  --distilled-cap 600 (geography raised to 3000 via DOMAIN_CAP_OVERRIDES)"
    echo "================================================================"

    python3 scripts/prepare_multibranch_data.py \
        --distilled "$V22_DISTILLED" \
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

# Gate 7 (v22-specific): geography (kept from v21) + identity.person.full_name
# (the v22-specific lift) must be present with meaningful volume.
geo_types = [k for k in label_counts if k.startswith('geography.')]
geo_total = sum(label_counts[k] for k in geo_types)
print(f'Gate 7a (v21 geography retained): Geography rows = {geo_total} (gate: >=2000)')
if geo_total < 2000:
    print(f'FAIL: geography rows {geo_total} < 2000 — v21 lift lost in blending')
    sys.exit(1)
print(f'  PASS — {geo_total} geography rows across {len(geo_types)} types')

full_name_rows = label_counts.get('identity.person.full_name', 0)
print(f'Gate 7b (v22 boundary lift): full_name rows = {full_name_rows} (gate: >=200)')
if full_name_rows < 200:
    print(f'FAIL: full_name rows {full_name_rows} < 200 — v22 boundary signal lost after cap')
    sys.exit(1)
print(f'  PASS — {full_name_rows} full_name rows present')
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
        ACC=$(python3 -c "import json; print(json.load(open('models/$name/results.json')).get('best_val_acc', 0))" 2>/dev/null || echo "0")
        echo "  $name: val_acc=$ACC"
        IS_BETTER=$(python3 -c "print(1 if float('$ACC') > float('$BEST_ACC') else 0)")
        if [[ "$IS_BETTER" == "1" ]]; then
            BEST="$name"
            BEST_ACC="$ACC"
        fi
    fi
done
echo "Best seed: $BEST (val_acc=$BEST_ACC)"
echo "(models/default NOT swapped — symlink update is reviewer's call, see ac-07)"

# ── Step 4: Summary ──────────────────────────────────────────────────
PIPELINE_END=$(date +%s)
TOTAL_ELAPSED=$(( (PIPELINE_END - PIPELINE_START) / 60 ))

echo "================================================================"
echo " v22 Boundary Retrain — Summary"
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
echo "Next steps (spec ac-06, ac-07, ac-08):"
echo "  1. Cherry-pick (already identified above): $BEST"
echo "  2. Swap models/default to the cherry-pick:"
echo "       ln -sfn $BEST models/default"
echo "  3. Run full corpus pass to a fresh output dir:"
echo "       source eval/gittables/.venv/bin/activate"
echo "       python3 scripts/gittables_corpus_pass.py --jobs 16 --execute --out-dir output/corpus-pass-v22  # ~7h"
echo "       python3 scripts/gittables_corpus_pass.py --fill-ydf --out-dir output/corpus-pass-v22          # ~25min"
echo "  4. Write four-way cell deltas (v19/v20/v21/v22) to output/corpus-pass-v22/cell_deltas.md."
echo "  5. Run branch-ablation diagnostic (ac-06):"
echo "       python3 scripts/branch_ablation_diagnostic.py --v21 sherlock-v21-geonames-geography-relu-s42 --v22 $BEST"
echo "  6. Read ac-08 band: cell-2 v22/v19 ≤ 0.80 = Met; 0.80-0.90 = Partial; > 0.90 = Failed."
echo ""
echo "Log: $LOG_FILE"
echo "================================================================"
