#!/usr/bin/env bash
# scripts/sweep_v18.sh — v18 multi-seed training sweep (data-seed discipline)
#
# Spec: orbit/specs/2026-04-21-v18-retrain/spec.yaml (v1.3)
#   — ac-02 (sweep script skeleton + prep instrumentation)
#   — ac-04 (row-hash firewall log markers)
#   — ac-05 (3-seed sweep executes cleanly)
#   — ac-06 (training gate, decision 0053)
# Decisions: 0060 (v3 corpus base), 0061 (data-seed discipline), 0053 (train gate).
#
# Key differences vs sweep_v17.sh:
#   • Single prep invocation OUTSIDE the seed loop (decision 0061). Prep runs
#     once with --seed 42 (DATA_SEED=42), writes ONE .ftmb that all 3 training
#     seeds share. FTMB NOT deleted between training runs — only after the
#     last training run completes. Saves ~60 min + 1.9 GB per sweep vs v17.
#   • v3 corpus base only (decision 0060). No --distilled-v4-dir. v4 UA /
#     LOINC adoption deferred to a follow-up card.
#   • Prep emits 8 log markers for ac-04 / ac-05 verification:
#       corpus_base, pre_filter_rows, row_hash_overlap, post_filter_rows,
#       hash_filter_active: true, leaked_rows_after_filter,
#       eval_hash_table_sha256, n_sibling_headers.
#     Markers are emitted by the patched prep script (ac-02).
#   • Per-seed eval writes to models/sherlock-v18-seed-{SEED}/eval/report.md
#     (ac-05 absolute path).
#
# Training gate (decision 0053) unchanged:
#   val_acc < 88%       → REJECT
#   88% ≤ val_acc < 91.2% → FLAG_MANUAL
#   val_acc ≥ 91.2%     → AUTO_ACCEPT
#
# Usage:
#   ./scripts/sweep_v18.sh                # Seeds 42,43,44 (default)
#   ./scripts/sweep_v18.sh 42             # Single seed
#
# Output:
#   output/multibranch-training/v18.ftmb       — single shared prep artefact
#   models/sherlock-v18-seed-{42,43,44}/       — per-seed model dirs
#   models/sherlock-v18-seed-{SEED}/eval/      — per-seed eval outputs
#   results/sweep-v18.log                      — full tee'd log (ac-04)
#   results/sweep-v18-summary.csv              — seed,val_acc,eval_*,gate_status

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

# Training gate thresholds (decision 0053)
FLOOR_VAL_ACC=0.88
AUTO_ACCEPT_VAL_ACC=0.912

# Data seed (decision 0061 — fixed per sweep)
DATA_SEED=42

# Default training seeds
if [[ $# -eq 0 ]]; then
    SEEDS=(42 43 44)
else
    SEEDS=("$@")
fi

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/sweep-v18.log"
SUMMARY_FILE="$LOG_DIR/sweep-v18-summary.csv"

# Tee all output to log (ac-04 verification reads this log)
exec > >(tee -a "$LOG_FILE") 2>&1

echo "================================================================"
echo " v18 Multi-Seed Sweep (data-seed discipline)"
echo " Started: $(date)"
echo " Data seed: $DATA_SEED (fixed per decision 0061)"
echo " Training seeds: ${SEEDS[*]}"
echo " Corpus base: v3 (decision 0060)"
echo " Training gate: floor=${FLOOR_VAL_ACC}  auto-accept=${AUTO_ACCEPT_VAL_ACC}"
echo " Expected time: ~30min prep + ${#SEEDS[@]} × 2.5h train + eval"
echo "================================================================"
echo ""

SWEEP_START=$(date +%s)

# ─── Pre-flight: build once ────────────────────────────────────────
echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"
echo ""

# ─── Prerequisites (v3 corpus base only — no v4 loaders needed) ─────
DISTILLED_FILE="output/distillation-v3/sherlock_distilled.csv.gz"
if [[ ! -f "$DISTILLED_FILE" ]]; then
    echo "FAIL: v3 distilled data not found at $DISTILLED_FILE"
    exit 1
fi
if [[ ! -f models/model2vec/model.safetensors ]]; then
    echo "FAIL: Model2Vec not found"
    exit 1
fi
if [[ ! -f models/sibling-context/model.safetensors ]]; then
    echo "FAIL: Sibling-context model not found"
    exit 1
fi
MODEL_CONFIG="models/sherlock-v13-config.json"
if [[ ! -f "$MODEL_CONFIG" ]]; then
    echo "FAIL: Model config not found at $MODEL_CONFIG"
    exit 1
fi
if [[ ! -f "eval/row_hashes.tsv" ]]; then
    echo "FAIL: eval/row_hashes.tsv not found (leakage firewall requires it)"
    exit 1
fi
echo "[Pre-flight] All prerequisites OK"
echo ""

# ─── Single prep invocation (decision 0061) ─────────────────────────
mkdir -p "output/multibranch-training"
FTMB_FILE="output/multibranch-training/v18.ftmb"
echo "[Prep] Single prep run with DATA_SEED=$DATA_SEED (decision 0061)"
echo "[Prep] Output: $FTMB_FILE"
echo "[Prep] Corpus base: v3 (decision 0060)"
echo ""

# Clean any stale prep artefact
rm -f "$FTMB_FILE"

# ac-04 / ac-05 instrumentation: the prep script emits
#   corpus_base, pre_filter_rows, row_hash_overlap, post_filter_rows,
#   hash_filter_active: true, leaked_rows_after_filter,
#   eval_hash_table_sha256, n_sibling_headers
# into stdout (captured by the top-level tee into results/sweep-v18.log).
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
    --seed "$DATA_SEED" \
    --workers 8

echo ""
echo "[Prep] Data prep complete. FTMB reused across all ${#SEEDS[@]} training seeds."
echo ""

# Verify the required ac-04 log markers are present in the log so far
# (defence-in-depth: fail early if instrumentation is missing)
REQUIRED_MARKERS=(
    "corpus_base:"
    "pre_filter_rows:"
    "row_hash_overlap:"
    "post_filter_rows:"
    "hash_filter_active: true"
    "leaked_rows_after_filter:"
    "eval_hash_table_sha256:"
    "n_sibling_headers:"
)
for marker in "${REQUIRED_MARKERS[@]}"; do
    if ! grep -q "$marker" "$LOG_FILE"; then
        echo "FAIL: ac-04/ac-05 required log marker missing: $marker"
        echo "      Prep-script instrumentation (ac-02) is incomplete."
        exit 1
    fi
done
echo "[Prep] All 8 required log markers present in $LOG_FILE (ac-04/ac-05)."
echo ""

# ─── Summary CSV header ─────────────────────────────────────────────
echo "seed,val_accuracy,eval_correct,eval_total,eval_pct,gate_status,time_secs" > "$SUMMARY_FILE"

declare -a SEED_VAL_ACCS=()
declare -a SEED_GATE_STATUS=()

# ─── Training loop (FTMB shared across seeds) ───────────────────────
for SEED in "${SEEDS[@]}"; do
    MODEL_DIR="models/sherlock-v18-seed-${SEED}"

    echo "================================================================"
    echo " Training seed $SEED — Starting: $(date)"
    echo " Model: $MODEL_DIR"
    echo " Shared data: $FTMB_FILE (DATA_SEED=$DATA_SEED, reused)"
    echo "================================================================"

    # Clean previous artefacts for this seed (but NOT the shared FTMB)
    rm -rf "$MODEL_DIR"

    SEED_START=$(date +%s)

    # --- Train ---
    cargo run --bin finetype --no-default-features --features metal --release -- \
        train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$MODEL_DIR" \
        --model-config "$MODEL_CONFIG" \
        --epochs 100 \
        --batch-size 32 \
        --lr 0.0001 \
        --weight-decay 0.0001 \
        --dropout 0.35 \
        --seed "$SEED" \
        --head flat \
        --patience 15 \
        2>&1

    SEED_END=$(date +%s)
    SEED_ELAPSED=$(( SEED_END - SEED_START ))

    echo "[Seed $SEED] Training complete in ${SEED_ELAPSED}s"

    # Inject type_index_keys into config.json
    SAVED_CONFIG="$MODEL_DIR/config.json"
    if [[ -f "$SAVED_CONFIG" ]] && ! grep -q '"type_index_keys"' "$SAVED_CONFIG"; then
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

    # Read best val_accuracy
    VAL_ACC=$(python3 -c "
import json
with open('$MODEL_DIR/results.json') as f:
    epochs = json.load(f)
best = max(epochs, key=lambda e: e['val_accuracy'])
print(f'{best[\"val_accuracy\"]:.4f}')
")
    echo "[Seed $SEED] Best val_accuracy: $VAL_ACC"

    # --- Training gate (decision 0053) ---
    GATE_STATUS=$(python3 -c "
v = float('$VAL_ACC')
if v < $FLOOR_VAL_ACC:
    print('REJECT')
elif v < $AUTO_ACCEPT_VAL_ACC:
    print('FLAG_MANUAL')
else:
    print('AUTO_ACCEPT')
")
    SEED_VAL_ACCS+=("$VAL_ACC")
    SEED_GATE_STATUS+=("$GATE_STATUS")

    case "$GATE_STATUS" in
        REJECT)
            echo "[Seed $SEED] ⛔ REJECTED — val_acc $VAL_ACC < ${FLOOR_VAL_ACC} floor."
            echo "             Skipping profile eval. Model retained at $MODEL_DIR."
            echo "$SEED,$VAL_ACC,0,0,0.0,REJECT,${SEED_ELAPSED}" >> "$SUMMARY_FILE"
            echo "[Seed $SEED] Done: $(date)"
            echo ""
            continue
            ;;
        FLAG_MANUAL)
            echo "[Seed $SEED] ⚠️  FLAGGED — val_acc $VAL_ACC in [${FLOOR_VAL_ACC}, ${AUTO_ACCEPT_VAL_ACC})."
            ;;
        AUTO_ACCEPT)
            echo "[Seed $SEED] ✅ AUTO-ACCEPTED — val_acc $VAL_ACC ≥ ${AUTO_ACCEPT_VAL_ACC}."
            ;;
    esac

    # --- Per-seed eval (ac-05 absolute path) ---
    echo "[Seed $SEED] Running profile eval..."
    rm -f eval/eval_output/profile_results.csv
    FINETYPE_MODEL="$MODEL_DIR" make eval-report 2>&1

    EVAL_SCORE=$(python3 -c "
import re
with open('eval/eval_output/report.md') as f:
    text = f.read()
m = re.search(r'Profile label accuracy \| (\d+)/(\d+)', text)
if m:
    print(f'{m.group(1)} {m.group(2)}')
else:
    print('0 0')
")
    EVAL_CORRECT=$(echo "$EVAL_SCORE" | awk '{print $1}')
    EVAL_TOTAL=$(echo "$EVAL_SCORE" | awk '{print $2}')
    EVAL_PCT=$(python3 -c "print(f'{int($EVAL_CORRECT)/int($EVAL_TOTAL)*100:.1f}')")

    # Save per-seed eval artefacts to absolute path (ac-05)
    mkdir -p "$MODEL_DIR/eval"
    cp eval/eval_output/profile_results.csv "$MODEL_DIR/eval/" 2>/dev/null || true
    cp eval/eval_output/report.md "$MODEL_DIR/eval/" 2>/dev/null || true

    echo "[Seed $SEED] Eval: $EVAL_CORRECT/$EVAL_TOTAL ($EVAL_PCT%)  [$GATE_STATUS]"
    echo "$SEED,$VAL_ACC,$EVAL_CORRECT,$EVAL_TOTAL,$EVAL_PCT,$GATE_STATUS,${SEED_ELAPSED}" >> "$SUMMARY_FILE"
    echo ""
done

# ─── All-seeds-fail-floor halt check (ac-06) ────────────────────────
ALL_REJECT=true
for gs in "${SEED_GATE_STATUS[@]}"; do
    if [[ "$gs" != "REJECT" ]]; then
        ALL_REJECT=false
        break
    fi
done

SWEEP_END=$(date +%s)
ELAPSED=$(( (SWEEP_END - SWEEP_START) / 60 ))

echo "================================================================"
echo " Sweep Complete"
echo " Finished: $(date)"
echo " Total elapsed: ${ELAPSED} minutes"
echo ""
echo " Results:"
cat "$SUMMARY_FILE" | column -t -s,
echo "================================================================"

# ─── Clean up shared FTMB after ALL seeds complete ──────────────────
if [[ -f "$FTMB_FILE" ]]; then
    echo "[Cleanup] Removing shared FTMB: $FTMB_FILE"
    rm -f "$FTMB_FILE"
fi

if $ALL_REJECT; then
    echo ""
    echo "⛔ HALT (ac-06 all-seeds-fail-floor): all ${#SEEDS[@]} seeds scored"
    echo "   below val_acc ${FLOOR_VAL_ACC}. v18 is NOT promoted."
    echo ""
    echo "   Investigate: data corruption, label_remap break, config drift."
    echo "   Document the halt + investigation path in progress.md with ≥3"
    echo "   hypotheses (symptom/testable_prediction/falsification_step/"
    echo "   estimated_cost) before any retry. See decisions 0053, 0054."
    exit 1
fi

echo ""
echo "Next step (ac-07): winner selection. Rule:"
echo "  highest profile-eval score > highest val_acc > lowest seed number."
echo "  Winner's score must be ≥ 297/352 AND per-domain regression ≤ 3."
echo "  If winner fails the gate: v18 is HELD (no candidate re-selection)."
echo ""
