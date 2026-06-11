#!/usr/bin/env bash
# scripts/overnight_v27_recall.sh — categorical+identifier recall candidate full retrain (ac-04).
#
# Spec 2026-06-11-categorical-identifier-recall-retrain. Runs ONLY after the
# destination-drift proxy pre-check (ac-03) returns GO. Trains the candidate at
# full depth — 3 ReLU+BN seeds × 50 epochs — on the ALREADY-BUILT candidate FTMB
# (output/multibranch-training/v27-recall-blend.ftmb): the first RECALL-direction
# bet of the gold era — curated positives for the two largest gold miss-pools
# (categorical R 0.386, alphanumeric_id R 0.111) mined from v19's own corpus
# mistakes.
#
# Architecture is ReLU+BN ONLY (models/sherlock-v13-config.json) — the shipped
# v19 default's architecture, so this is a clean DATA bet against it, not an
# architecture comparison.
#
# The FTMB is built by build_v27_recall_distilled.py + prepare_multibranch_data.py
# with --type-cap categorical=3400 / alphanumeric_id=2400; this script reuses it
# (no data prep). It does NOT cherry-pick a seed or swap models/default — the
# gold corpus re-score (ac-04, scripts/score_gold_anchor.py) across all 3 seeds
# is the judge, and promotion (ac-07) is a separate reviewed decision behind the
# corpus-honest gate (ac-06, blocking).
#
# Usage:
#   scripts/overnight_v27_recall.sh                 # full 3-seed run
#   scripts/overnight_v27_recall.sh --epochs N      # override epoch cap
#   scripts/overnight_v27_recall.sh --dry-run       # show config, don't train
set -eo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"; mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v27-recall.log"

DRY_RUN=false
EPOCHS=50
PATIENCE=15
BATCH_SIZE=32

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)  DRY_RUN=true; shift ;;
        --epochs)   EPOCHS="$2"; shift 2 ;;
        --help|-h)  sed -n '2,/^set -/p' "$0" | grep '^#' | sed 's/^# \?//'; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

exec > >(tee -a "$LOG_FILE") 2>&1

RELU_CONFIG="models/sherlock-v13-config.json"
FTMB_FILE="output/multibranch-training/v27-recall-blend.ftmb"

# name|config|seed|lr|wd  — ReLU+BN, 3 seeds, identical hyperparameters to v19.
RUNS=(
    "sherlock-v27-recall-relu-s42|$RELU_CONFIG|42|0.0001|0.0001"
    "sherlock-v27-recall-relu-s43|$RELU_CONFIG|43|0.0001|0.0001"
    "sherlock-v27-recall-relu-s44|$RELU_CONFIG|44|0.0001|0.0001"
)

echo "================================================================"
echo " v27 categorical+identifier recall retrain (ac-04)"
echo " Started: $(date)   Host: $(hostname) — $(uname -m)"
echo " Architecture: ReLU+BN ($RELU_CONFIG)"
echo " Seeds:        42 43 44"
echo " Epochs:       $EPOCHS, patience: $PATIENCE, batch: $BATCH_SIZE"
echo " Data:         $FTMB_FILE (already built; proxy pre-check = GO)"
echo "================================================================"
echo ""

if [[ "$DRY_RUN" == "true" ]]; then
    echo "[DRY RUN] Would train ${#RUNS[@]} runs on $FTMB_FILE:"
    for run_def in "${RUNS[@]}"; do
        IFS='|' read -r name config seed lr wd <<< "$run_def"
        echo "  $name: seed=$seed lr=$lr wd=$wd -> models/$name/"
    done
    exit 0
fi

PIPELINE_START=$(date +%s)

# ── Pre-flight ────────────────────────────────────────────────────────
echo "[Pre-flight] Checking prerequisites..."
[[ -f "$RELU_CONFIG" ]] || { echo "FAIL: config not found: $RELU_CONFIG"; exit 1; }
grep -q '"n_classes": 240' "$RELU_CONFIG" || { echo "FAIL: n_classes must be 240"; exit 1; }
grep -q '"valid_dim": 240' "$RELU_CONFIG" || { echo "FAIL: valid_dim must be 240"; exit 1; }
[[ -f "$FTMB_FILE" ]] || { echo "FAIL: candidate FTMB not found: $FTMB_FILE"; exit 1; }
[[ -f models/model2vec/model.safetensors ]] || { echo "FAIL: Model2Vec missing"; exit 1; }
[[ -f models/sibling-context/model.safetensors ]] || { echo "FAIL: sibling-context missing"; exit 1; }
echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"
echo ""

# ── Step 1: FTMB audit gate (the candidate FTMB is pre-built) ─────────
echo "================================================================"
echo " Step 1: Pre-training Data Audit Gate"
echo "================================================================"
python3 -c "
import struct, sys
path = '$FTMB_FILE'
with open(path, 'rb') as f:
    assert f.read(4) == b'FTMB', 'Bad magic'
    version = struct.unpack('<I', f.read(4))[0]
    struct.unpack('<Q', f.read(8))[0]
    char_dim = struct.unpack('<H', f.read(2))[0]
    embed_dim = struct.unpack('<H', f.read(2))[0]
    stats_dim = struct.unpack('<H', f.read(2))[0]
    header_dim = struct.unpack('<H', f.read(2))[0]
    if version < 4:
        print(f'FAIL: FTMB version {version}, expected v4+'); sys.exit(1)
    n_groups = struct.unpack('<H', f.read(2))[0]
    struct.unpack('<H', f.read(2))[0]
    valid_dim = struct.unpack('<H', f.read(2))[0]
    if valid_dim != 240:
        print(f'FAIL: valid_dim={valid_dim}, expected 240'); sys.exit(1)
    print('Gate 1: valid_dim=240 — PASS')
    record_size = char_dim*4 + embed_dim*4 + stats_dim*4 + header_dim*4 + valid_dim*4
    label_counts = {}
    for g in range(n_groups):
        n_columns = struct.unpack('<H', f.read(2))[0]
        n_sib = struct.unpack('<H', f.read(2))[0]
        for _ in range(n_sib):
            name_len = struct.unpack('<H', f.read(2))[0]; f.read(name_len)
        for c in range(n_columns):
            label_len = struct.unpack('<H', f.read(2))[0]
            label = f.read(label_len).decode('utf-8')
            struct.unpack('<H', f.read(2))[0]; f.read(record_size)
            label_counts[label] = label_counts.get(label, 0) + 1
min_type = min(label_counts.values())
print(f'Gate 2: min type count = {min_type} (gate >=50)')
if min_type < 50: print('FAIL'); sys.exit(1)
print('  PASS')
total_types = len(label_counts)
print(f'Gate 3: total types = {total_types} (gate >=238)')
if total_types < 238: print('FAIL'); sys.exit(1)
print('  PASS')
geo_total = sum(v for k, v in label_counts.items() if k.startswith('geography.'))
print(f'Gate 4 (geography retained): {geo_total} rows (gate >=2000)')
if geo_total < 2000: print('FAIL: v19 geography lost in blend'); sys.exit(1)
print('  PASS')
cat_n = label_counts.get('representation.discrete.categorical', 0)
print(f'Gate 5 (categorical signal): {cat_n} rows (gate >=2500)')
if cat_n < 2500: print('FAIL: categorical positives collapsed under cap'); sys.exit(1)
print('  PASS')
aid_n = label_counts.get('representation.identifier.alphanumeric_id', 0)
print(f'Gate 6 (alphanumeric_id signal): {aid_n} rows (gate >=1500)')
if aid_n < 1500: print('FAIL: alnum positives collapsed under cap'); sys.exit(1)
print('  PASS')
for guard, floor in (('geography.location.city', 400),
                     ('representation.text.word', 200),
                     ('representation.text.entity_name', 400),
                     ('representation.boolean.terms', 200)):
    n = label_counts.get(guard, 0)
    print(f'Gate 7 (neighbour retained) {guard}: {n} rows (gate >={floor})')
    if n < floor: print(f'FAIL: neighbour {guard} starved by blend'); sys.exit(1)
print('  PASS')
print('')
print(f'Audit summary: {total_types} types, {sum(label_counts.values())} records')
print('ALL GATES PASSED')
"
echo ""

# ── Step 2: Training runs ─────────────────────────────────────────────
RUN_NAMES=(); RUN_STATUSES=(); RUN_TIMES=()
echo "================================================================"
echo " Step 2: Training — ${#RUNS[@]} runs"
echo " Started: $(date)"
echo "================================================================"
echo ""

RUN_NUM=0
for run_def in "${RUNS[@]}"; do
    IFS='|' read -r name config seed lr wd <<< "$run_def"
    RUN_NAMES+=("$name"); RUN_NUM=$((RUN_NUM + 1))
    MODEL_DIR="models/$name"
    echo "────────────────────────────────────────────────────────────"
    echo " Run $RUN_NUM/${#RUNS[@]}: $name (ReLU+BN)"
    echo " Config: $config | Seed: $seed | LR: $lr | WD: $wd | Started: $(date)"
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
        RUN_END=$(date +%s); RUN_ELAPSED=$(( (RUN_END - RUN_START) / 60 ))
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
        RUN_END=$(date +%s); RUN_ELAPSED=$(( (RUN_END - RUN_START) / 60 ))
        RUN_STATUSES+=("FAILED"); RUN_TIMES+=("$RUN_ELAPSED")
        echo "[Run $RUN_NUM] FAILED after ${RUN_ELAPSED} min — continuing"
    fi
    echo ""
done

# ── Step 3: Summary ──────────────────────────────────────────────────
PIPELINE_END=$(date +%s)
TOTAL_ELAPSED=$(( (PIPELINE_END - PIPELINE_START) / 60 ))
echo "================================================================"
echo " v27 recall retrain — Summary"
echo " Finished: $(date)   Total elapsed: ${TOTAL_ELAPSED} min"
echo "================================================================"
printf "%-36s %-10s %-10s\n" "Run" "Status" "Time(min)"
printf "%-36s %-10s %-10s\n" "---" "------" "---------"
for i in "${!RUN_NAMES[@]}"; do
    printf "%-36s %-10s %-10s\n" "${RUN_NAMES[$i]}" "${RUN_STATUSES[$i]:-UNKNOWN}" "${RUN_TIMES[$i]:-?}"
done
echo ""

OK_COUNT=0
for status in "${RUN_STATUSES[@]}"; do
    [[ "$status" == "OK" || "$status" == "SKIPPED" ]] && OK_COUNT=$((OK_COUNT + 1))
done
echo "Completed seeds: $OK_COUNT/${#RUNS[@]} (MADR 0066: all 3 required for the gate)"
echo ""
echo "Next (ac-04): gold corpus re-score per seed —"
echo "  scripts/score_gold_anchor.py predict/score vs the post-patch baseline"
echo "  (output/postal-veto/metrics_v19-postal-veto_2026-06-10.tsv, 635/931=0.682)."
echo "  Success: categorical recall >=0.55 (P>=0.80), alphanumeric_id recall"
echo "  >=0.45 (P>=0.80), headline >=0.70, no support>=10 label -5pp."
echo "Next (ac-05): full-label-space post-train drift vs v19 + rare-type scoreboard."
echo "Next (ac-06): corpus-honest gate (BLOCKING). models/default NOT swapped here."
echo ""
echo "Log: $LOG_FILE"
echo "================================================================"
