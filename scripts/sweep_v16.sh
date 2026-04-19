#!/usr/bin/env bash
# scripts/sweep_v16.sh — Multi-seed training sweep for v16
#
# Runs N training jobs sequentially with different seeds, evaluates each,
# and promotes the best model to models/sherlock-v16.
#
# Usage:
#   ./scripts/sweep_v16.sh                    # Seeds 42,43,44 (default)
#   ./scripts/sweep_v16.sh 42 43 44 45 46     # Custom seeds
#
# Each run:
#   1. Prepares fresh FTMB data (seed affects synthetic data sampling)
#   2. Trains for 100 epochs with patience 15
#   3. Runs profile eval
#   4. Records score
#
# After all runs, the best model (by eval score) is promoted.
#
# Estimated time: ~2.5 hours per seed on M1 Pro with Metal
#
# Output:
#   models/sherlock-v16-seed-NN/    — Per-seed model dirs
#   models/sherlock-v16/            — Symlink to best seed
#   results/sweep-v16.log           — Full log
#   results/sweep-v16-summary.csv   — Seed, val_acc, eval_score
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

# Default seeds
if [[ $# -eq 0 ]]; then
    SEEDS=(42 43 44)
else
    SEEDS=("$@")
fi

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/sweep-v16.log"
SUMMARY_FILE="$LOG_DIR/sweep-v16-summary.csv"

# Tee all output to log
exec > >(tee -a "$LOG_FILE") 2>&1

echo "================================================================"
echo " v16 Multi-Seed Sweep"
echo " Started: $(date)"
echo " Seeds: ${SEEDS[*]}"
echo " Runs: ${#SEEDS[@]} × ~2.5h = ~$((${#SEEDS[@]} * 150 / 60))h estimated"
echo "================================================================"
echo ""

SWEEP_START=$(date +%s)

# Pre-flight: build once
echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"
echo ""

# Verify prerequisites (same checks as overnight script, run once)
DISTILLED_FILE="output/distillation-v3/sherlock_distilled.csv.gz"
if [[ ! -f "$DISTILLED_FILE" ]]; then
    echo "FAIL: Distilled data not found at $DISTILLED_FILE"
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
echo ""

# Write CSV header
echo "seed,val_accuracy,eval_correct,eval_total,eval_pct" > "$SUMMARY_FILE"

BEST_SEED=""
BEST_EVAL=0

for SEED in "${SEEDS[@]}"; do
    MODEL_DIR="models/sherlock-v16-seed-${SEED}"
    FTMB_FILE="output/multibranch-training/v16-seed-${SEED}.ftmb"

    echo "================================================================"
    echo " Seed $SEED — Starting: $(date)"
    echo " Model: $MODEL_DIR"
    echo " Data:  $FTMB_FILE"
    echo "================================================================"

    # Clean previous artifacts for this seed
    rm -rf "$MODEL_DIR"
    rm -f "$FTMB_FILE"

    # --- Data prep ---
    echo "[Seed $SEED] Preparing training data..."
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

    # Inject type_index_keys
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

    # Read val accuracy
    VAL_ACC=$(python3 -c "
import json
with open('$MODEL_DIR/results.json') as f:
    epochs = json.load(f)
best = max(epochs, key=lambda e: e['val_accuracy'])
print(f'{best[\"val_accuracy\"]:.4f}')
")
    echo "[Seed $SEED] Best val_accuracy: $VAL_ACC"

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

    echo "[Seed $SEED] Eval: $EVAL_CORRECT/$EVAL_TOTAL ($EVAL_PCT%)"
    echo "$SEED,$VAL_ACC,$EVAL_CORRECT,$EVAL_TOTAL,$EVAL_PCT" >> "$SUMMARY_FILE"

    # Track best
    if [[ "$EVAL_CORRECT" -gt "$BEST_EVAL" ]]; then
        BEST_EVAL="$EVAL_CORRECT"
        BEST_SEED="$SEED"
    fi

    echo ""
    echo "[Seed $SEED] Done: $(date)"
    echo ""

    # Clean up FTMB to save disk (each is ~950MB)
    rm -f "$FTMB_FILE"
done

# --- Promote best model ---

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
echo " Best: seed=$BEST_SEED with $BEST_EVAL/$EVAL_TOTAL"
echo "================================================================"

# Promote best seed to sherlock-v16
BEST_MODEL_DIR="models/sherlock-v16-seed-${BEST_SEED}"
V16_DIR="models/sherlock-v16"

if [[ "$BEST_EVAL" -ge 233 ]]; then
    echo ""
    echo "Best model meets target (>= 233/242). Promoting..."
    rm -rf "$V16_DIR"
    cp -r "$BEST_MODEL_DIR" "$V16_DIR"
    ln -sf sherlock-v16 models/default
    echo "  models/default -> sherlock-v16 (from seed $BEST_SEED)"
    echo ""
    echo "v16 is live. Run golden tests to verify:"
    echo "  cargo test -p finetype-cli --test cli_golden -- --ignored"
else
    echo ""
    echo "No model met the target (>= 233/242). Best was $BEST_EVAL/242 (seed $BEST_SEED)."
    echo "models/default remains on sherlock-v14."
    echo ""
    echo "Models are preserved at models/sherlock-v16-seed-*/ for inspection."
fi
