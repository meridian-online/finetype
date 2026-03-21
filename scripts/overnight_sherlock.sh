#!/usr/bin/env bash
# scripts/overnight_sherlock.sh — Run the full Sherlock ablation pipeline overnight.
#
# Runs on M1 Pro with Metal acceleration. Expects:
#   - Distilled data at output/distillation-v3/sherlock_distilled.csv.gz
#   - Model2Vec at models/model2vec/
#   - Hierarchical head implemented in multi_branch.rs
#
# Usage:
#   ./scripts/overnight_sherlock.sh           # Full pipeline
#   ./scripts/overnight_sherlock.sh --skip-baseline  # Skip PRE-1 baseline retraining
#   ./scripts/overnight_sherlock.sh --flat-only      # Only flat experiments (AC-6a/b)
#
# Output:
#   models/char-cnn-v16-baseline/  — PRE-1 baseline CharCNN
#   output/multibranch-training/   — .ftmb training data
#   models/sherlock-v1-flat/       — AC-6a: flat, no sibling-context
#   models/sherlock-v1-hier/       — AC-6c: hierarchical, no sibling-context
#   results/sherlock-ablation.log  — Full log
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/sherlock-ablation.log"

SKIP_BASELINE=false
FLAT_ONLY=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-baseline) SKIP_BASELINE=true; shift ;;
        --flat-only)     FLAT_ONLY=true; shift ;;
        --help|-h)
            sed -n '2,/^set -/p' "$0" | grep '^#' | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# Tee all output to log
exec > >(tee -a "$LOG_FILE") 2>&1

echo "════════════════════════════════════════════════════════════════"
echo " Sherlock Ablation Pipeline"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo "════════════════════════════════════════════════════════════════"
echo ""

PIPELINE_START=$(date +%s)

# ─── Pre-flight checks ────────────────────────────────────────────

echo "[Pre-flight] Checking prerequisites..."

if [[ ! -f output/distillation-v3/sherlock_distilled.csv.gz ]]; then
    echo "FAIL: Distilled data not found at output/distillation-v3/sherlock_distilled.csv.gz"
    exit 1
fi

if [[ ! -d models/model2vec ]]; then
    echo "FAIL: Model2Vec not found at models/model2vec/"
    exit 1
fi

echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"
echo ""

# ─── Step 1: PRE-1 Baseline CharCNN ──────────────────────────────

if [[ "$SKIP_BASELINE" == "false" ]]; then
    echo "════════════════════════════════════════════════════════════════"
    echo " Step 1/4: PRE-1 — Baseline CharCNN (blend-30-70)"
    echo " Started: $(date)"
    echo "════════════════════════════════════════════════════════════════"

    mkdir -p output/baseline-training

    echo "[PRE-1] Generating blended training data..."
    python3 scripts/prepare_spike_data.py \
        --distilled output/distillation-v3/sherlock_distilled.csv.gz \
        --ratio-distilled 0.3 \
        --samples-per-type 1500 \
        --seed 42 \
        --output output/baseline-training/blend-30-70.ndjson

    echo ""
    echo "[PRE-1] Training CharCNN baseline..."
    ./scripts/train.sh \
        --data output/baseline-training/blend-30-70.ndjson \
        --size large \
        --epochs 10 \
        --seed 42 \
        --model-name char-cnn-v16-baseline

    echo ""
    echo "[PRE-1] Evaluating baseline..."
    ./scripts/eval.sh --model models/char-cnn-v16-baseline || echo "WARN: Eval failed, continuing..."

    echo ""
    echo "[PRE-1] Complete: $(date)"
    echo ""
else
    echo "[PRE-1] Skipped (--skip-baseline)"
    echo ""
fi

# ─── Step 2: AC-5 — Prepare Multi-Branch Training Data ───────────

echo "════════════════════════════════════════════════════════════════"
echo " Step 2/4: AC-5 — Prepare Multi-Branch Training Data (.ftmb)"
echo " Started: $(date)"
echo "════════════════════════════════════════════════════════════════"

FTMB_FILE="output/multibranch-training/blend-30-70.ftmb"
mkdir -p output/multibranch-training

if [[ -f "$FTMB_FILE" ]]; then
    echo "[AC-5] FTMB file already exists, verifying..."
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    echo "[AC-5] Using existing FTMB file"
else
    echo "[AC-5] Dry run..."
    python3 scripts/prepare_multibranch_data.py --dry-run

    echo ""
    echo "[AC-5] Full extraction..."
    python3 scripts/prepare_multibranch_data.py \
        --distilled output/distillation-v3/sherlock_distilled.csv.gz \
        --finetype ./target/release/finetype \
        --output "$FTMB_FILE" \
        --samples-per-type 1500 \
        --ratio-distilled 0.3 \
        --seed 42 \
        --workers 8

    echo ""
    echo "[AC-5] Verifying output..."
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
fi

echo ""
echo "[AC-5] Complete: $(date)"
echo ""

# ─── Step 3: AC-6 — Ablation Experiments ─────────────────────────

echo "════════════════════════════════════════════════════════════════"
echo " Step 3/4: AC-6 — Ablation Training Experiments"
echo " Started: $(date)"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Note: AC-6b and AC-6d (with sibling-context) require sibling-context-enriched
# embedding features. This requires a second .ftmb file with preprocessed headers.
# For now, we train the two non-sibling experiments. Sibling-context experiments
# are deferred to a follow-up session where the data prep pipeline is extended.
#
# TODO: AC-6b (flat + sibling) and AC-6d (hier + sibling)

# AC-6a: Flat head, no sibling-context
echo "── AC-6a: Flat head, no sibling-context ──────────────────────"
echo "Started: $(date)"

if [[ -f models/sherlock-v1-flat/model.safetensors ]]; then
    echo "[AC-6a] Model already exists, skipping training"
else
    # The train-multi-branch CLI command needs to be wired up.
    # For now, use cargo run directly with the training function.
    cargo run --bin finetype --no-default-features --features metal --release -- \
        train-multi-branch \
        --data "$FTMB_FILE" \
        --output models/sherlock-v1-flat \
        --epochs 10 \
        --batch-size 32 \
        --lr 0.0001 \
        --weight-decay 0.0001 \
        --dropout 0.35 \
        --seed 42 \
        --head flat \
        --patience 10 \
    2>&1
fi

echo ""
echo "AC-6a complete: $(date)"
echo ""

# AC-6c: Hierarchical head, no sibling-context
if [[ "$FLAT_ONLY" == "false" ]]; then
    echo "── AC-6c: Hierarchical head, no sibling-context ──────────────"
    echo "Started: $(date)"

    if [[ -f models/sherlock-v1-hier/model.safetensors ]]; then
        echo "[AC-6c] Model already exists, skipping training"
    else
        cargo run --bin finetype --no-default-features --features metal --release -- \
            train-multi-branch \
            --data "$FTMB_FILE" \
            --output models/sherlock-v1-hier \
            --epochs 10 \
            --batch-size 32 \
            --lr 0.0001 \
            --weight-decay 0.0001 \
            --dropout 0.35 \
            --seed 42 \
            --head hierarchical \
            --patience 10 \
        2>&1
    fi

    echo ""
    echo "AC-6c complete: $(date)"
    echo ""
fi

# ─── Step 4: Evaluation ──────────────────────────────────────────

echo "════════════════════════════════════════════════════════════════"
echo " Step 4/4: Evaluation"
echo " Started: $(date)"
echo "════════════════════════════════════════════════════════════════"
echo ""

for model_dir in models/sherlock-v1-flat models/sherlock-v1-hier; do
    if [[ -f "$model_dir/model.safetensors" ]]; then
        echo "── Evaluating $model_dir ──"
        ./scripts/eval.sh --model "$model_dir" || echo "WARN: Eval failed for $model_dir"
        echo ""
    else
        echo "── Skipping $model_dir (no model.safetensors) ──"
        echo ""
    fi
done

# ─── Summary ─────────────────────────────────────────────────────

PIPELINE_END=$(date +%s)
TOTAL_ELAPSED=$((PIPELINE_END - PIPELINE_START))
TOTAL_HOURS=$((TOTAL_ELAPSED / 3600))
TOTAL_MINS=$(( (TOTAL_ELAPSED % 3600) / 60))

echo "════════════════════════════════════════════════════════════════"
echo " Pipeline Complete"
echo " Finished: $(date)"
echo " Total time: ${TOTAL_HOURS}h ${TOTAL_MINS}m"
echo " Log: $LOG_FILE"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "Next steps:"
echo "  1. Check results in $LOG_FILE"
echo "  2. Compare models: eval/eval_output/report.md"
echo "  3. If results look good, wire up pipeline integration (AC-9)"
echo ""
echo "Note: AC-6b (flat+sibling) and AC-6d (hier+sibling) need"
echo "sibling-context-enriched .ftmb data — deferred to follow-up."
