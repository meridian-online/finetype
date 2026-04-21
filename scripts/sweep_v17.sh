#!/usr/bin/env bash
# scripts/sweep_v17.sh — Multi-seed training sweep for v17
#
# Spec: specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml (v1.3)
#   — acceptance_criteria ac-09 (sweep + training gate).
#
# Differences from sweep_v16.sh:
#   • Passes v4 distilled dir to the prep script (user_agent + LOINC
#     real-data layered on top of v3).
#   • Training gate (decision 0053):
#       val_acc < 88%  → REJECT (not promoted, no eval run for that seed)
#       88% ≤ val_acc < 91.2% → FLAG for manual review (eval still runs)
#       val_acc ≥ 91.2% → auto-accept
#   • NO auto-promotion. The winner is chosen in ac-11 (manual step),
#     not by this script. We just produce 3 per-seed model dirs + eval
#     outputs + a summary CSV.
#   • All-seeds-fail-floor halt path: if all 3 seeds' best val_acc < 88%,
#     emit a halt message and exit non-zero before touching eval.
#
# Usage:
#   ./scripts/sweep_v17.sh                    # Seeds 42,43,44 (default)
#   ./scripts/sweep_v17.sh 42 43              # Custom seed subset
#
# Output:
#   models/sherlock-v17-seed-NN/    — Per-seed model dirs (results.json,
#                                     epochs.jsonl, config.json,
#                                     model.safetensors, eval/report.md)
#   results/sweep-v17.log           — Full log (tee'd)
#   results/sweep-v17-summary.csv   — seed,val_acc,eval_correct,
#                                     eval_total,eval_pct,gate_status
#
# Estimated time: ~2.5-3h per seed on M1 Pro with Metal
#                 (v16 precedent + v4 data overhead).
#
# Training gate rationale: v14 shipped at 91.2% val_acc; 88% is ~3pp
# below (≈1σ on typical training variance). The true promotion gate is
# profile eval (ac-11), not val_acc.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

# Training gate thresholds (decision 0053)
FLOOR_VAL_ACC=0.88       # reject below
AUTO_ACCEPT_VAL_ACC=0.912 # auto-accept at or above; between → flag

# Default seeds
if [[ $# -eq 0 ]]; then
    SEEDS=(42 43 44)
else
    SEEDS=("$@")
fi

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/sweep-v17.log"
SUMMARY_FILE="$LOG_DIR/sweep-v17-summary.csv"

# Tee all output to log
exec > >(tee -a "$LOG_FILE") 2>&1

echo "================================================================"
echo " v17 Multi-Seed Sweep"
echo " Started: $(date)"
echo " Seeds: ${SEEDS[*]}"
echo " Training gate: floor=${FLOOR_VAL_ACC}  auto-accept=${AUTO_ACCEPT_VAL_ACC}"
echo " Expected time: ~${#SEEDS[@]} × 2.5h = ~$((${#SEEDS[@]} * 150 / 60))h"
echo "================================================================"
echo ""

SWEEP_START=$(date +%s)

# Pre-flight: build once
echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"
echo ""

# Verify prerequisites
DISTILLED_FILE="output/distillation-v3/sherlock_distilled.csv.gz"
V4_DIR="output/distillation-v4"
if [[ ! -f "$DISTILLED_FILE" ]]; then
    echo "FAIL: v3 distilled data not found at $DISTILLED_FILE"
    exit 1
fi
if [[ ! -f "$V4_DIR/user_agent.csv" ]] || [[ ! -f "$V4_DIR/loinc.csv" ]]; then
    echo "FAIL: v4 distilled CSVs missing — run loaders first:"
    echo "  python3 $V4_DIR/loaders/user_agent.py"
    echo "  python3 $V4_DIR/loaders/loinc.py"
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
echo "[Pre-flight] All prerequisites OK"
echo "  v4: $(wc -l < "$V4_DIR/user_agent.csv") UA rows, $(wc -l < "$V4_DIR/loinc.csv") LOINC rows"
echo ""

# Write CSV header
echo "seed,val_accuracy,eval_correct,eval_total,eval_pct,gate_status" > "$SUMMARY_FILE"

# Track per-seed val_acc for all-seeds-fail-floor halt check
declare -a SEED_VAL_ACCS=()
declare -a SEED_GATE_STATUS=()

for SEED in "${SEEDS[@]}"; do
    MODEL_DIR="models/sherlock-v17-seed-${SEED}"
    FTMB_FILE="output/multibranch-training/v17-seed-${SEED}.ftmb"

    echo "================================================================"
    echo " Seed $SEED — Starting: $(date)"
    echo " Model: $MODEL_DIR"
    echo " Data:  $FTMB_FILE"
    echo "================================================================"

    # Clean previous artefacts for this seed
    rm -rf "$MODEL_DIR"
    rm -f "$FTMB_FILE"

    # --- Data prep (v17: v3 + v4 distilled) ---
    echo "[Seed $SEED] Preparing training data (v3 + v4)..."
    python3 scripts/prepare_multibranch_data.py \
        --distilled "$DISTILLED_FILE" \
        --distilled-v4-dir "$V4_DIR" \
        --v4-column-size 100 \
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
        --seed "$SEED" \
        --workers 8
    echo "[Seed $SEED] Data prep complete"

    # --- Train ---
    echo "[Seed $SEED] Training (100 epochs, patience 15)..."
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

    echo "[Seed $SEED] Training complete"

    # Inject type_index_keys (multi-branch expects this in config.json)
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
            echo "             Skipping profile eval. Model retained at $MODEL_DIR for inspection."
            echo "$SEED,$VAL_ACC,0,0,0.0,REJECT" >> "$SUMMARY_FILE"
            echo ""
            echo "[Seed $SEED] Done: $(date)"
            echo ""
            rm -f "$FTMB_FILE"
            continue
            ;;
        FLAG_MANUAL)
            echo "[Seed $SEED] ⚠️  FLAGGED for manual review — val_acc $VAL_ACC in [${FLOOR_VAL_ACC}, ${AUTO_ACCEPT_VAL_ACC})."
            echo "             Profile eval will run, but ac-11 promotion requires explicit Hugh sign-off."
            ;;
        AUTO_ACCEPT)
            echo "[Seed $SEED] ✅ AUTO-ACCEPTED — val_acc $VAL_ACC ≥ ${AUTO_ACCEPT_VAL_ACC}."
            ;;
    esac

    # --- Eval ---
    echo "[Seed $SEED] Running profile eval..."
    rm -f eval/eval_output/profile_results.csv
    # NB: profile_eval.sh reads FINETYPE_MODEL (passed to CLI as --model).
    # FINETYPE_MODEL_DIR is only honoured by the DuckDB extension.
    FINETYPE_MODEL="$MODEL_DIR" make eval-report 2>&1

    # Extract eval score
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

    # Save eval results to model dir
    mkdir -p "$MODEL_DIR/eval"
    cp eval/eval_output/profile_results.csv "$MODEL_DIR/eval/" 2>/dev/null || true
    cp eval/eval_output/report.md "$MODEL_DIR/eval/" 2>/dev/null || true

    echo "[Seed $SEED] Eval: $EVAL_CORRECT/$EVAL_TOTAL ($EVAL_PCT%)  [$GATE_STATUS]"
    echo "$SEED,$VAL_ACC,$EVAL_CORRECT,$EVAL_TOTAL,$EVAL_PCT,$GATE_STATUS" >> "$SUMMARY_FILE"

    echo ""
    echo "[Seed $SEED] Done: $(date)"
    echo ""

    # Clean up FTMB to save disk (each is ~950MB)
    rm -f "$FTMB_FILE"
done

# --- All-seeds-fail-floor halt check (ac-09) ---
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
echo ""
echo "================================================================"

if $ALL_REJECT; then
    echo ""
    echo "⛔ HALT (ac-09 all-seeds-fail-floor): all ${#SEEDS[@]} seeds scored"
    echo "   below val_acc ${FLOOR_VAL_ACC}. v17 is NOT promoted. v0.6.18"
    echo "   is NOT released."
    echo ""
    echo "   Investigate: data corruption, label_remap break, config drift."
    echo "   Models retained at models/sherlock-v17-seed-*/ for inspection."
    echo ""
    echo "   Document the halt + investigation path in progress.md before"
    echo "   any retry. See decision 0053."
    exit 1
fi

echo ""
echo "Next step (ac-11): review per-seed results. Winner selection rule:"
echo "  highest profile-eval score > highest val_acc > lowest seed number."
echo ""
echo "  If the winner's val_acc is in [${FLOOR_VAL_ACC}, ${AUTO_ACCEPT_VAL_ACC}):"
echo "    require explicit Hugh sign-off on promotion PR (ac-11 manual-review)."
echo ""
echo "  Promotion gate: winner_score ≥ max(235, v16_baseline_score)."
echo "  v16 baseline pinned in specs/2026-04-20-distilled-data-relabel-7-types/v16-baseline.md"
echo ""
echo "  Do NOT auto-promote. Proceed via ac-12 manual promotion PR."
