#!/usr/bin/env bash
# scripts/overnight_sherlock.sh — Overnight multi-branch training pipeline (v2)
#
# Runs on M1 Pro with Metal acceleration. Expects:
#   - Distilled data at output/distillation-v3/sherlock_distilled.csv.gz
#   - Model2Vec at models/model2vec/
#   - Label remap at data/label_remap.json
#
# Pipeline:
#   1. Prepare 300k+ column-level training data (.ftmb)
#   2. Train flat multi-branch model (20 epochs)
#   3. Train hierarchical multi-branch model (15 epochs)
#   4. Evaluate both models against Tier 1 profile eval
#
# Usage:
#   ./scripts/overnight_sherlock.sh           # Full pipeline
#   ./scripts/overnight_sherlock.sh --flat-only      # Skip hierarchical
#   ./scripts/overnight_sherlock.sh --skip-data       # Skip data prep (reuse existing .ftmb)
#
# Output:
#   output/multibranch-training/blend-30-70.ftmb  — Training data
#   models/sherlock-v2-flat/                       — Flat model + eval
#   models/sherlock-v2-hier/                       — Hierarchical model + eval
#   results/overnight-v2.log                       — Full log
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v2.log"

FLAT_ONLY=false
SKIP_DATA=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --flat-only)  FLAT_ONLY=true; shift ;;
        --skip-data)  SKIP_DATA=true; shift ;;
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
echo " Multi-Branch Overnight Pipeline v2"
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

if [[ ! -f data/label_remap.json ]]; then
    echo "WARN: Label remap not found at data/label_remap.json (proceeding without remap)"
fi

if ! command -v duckdb >/dev/null 2>&1; then
    echo "FAIL: duckdb CLI not found on PATH (required for eval summary)"
    echo "  Install: brew install duckdb  or  https://duckdb.org/docs/installation/"
    exit 1
fi

echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"
echo ""

# ─── Step 1: Prepare Training Data (M-3 + M-4 + M-5) ────────────

FTMB_FILE="output/multibranch-training/blend-30-70.ftmb"
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "════════════════════════════════════════════════════════════════"
    echo " Step 1/4: Data prep — SKIPPED (--skip-data, reusing existing)"
    echo "════════════════════════════════════════════════════════════════"
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    echo ""
else
    echo "════════════════════════════════════════════════════════════════"
    echo " Step 1/4: Prepare 300k+ Training Data (.ftmb)"
    echo " Started: $(date)"
    echo "════════════════════════════════════════════════════════════════"

    # Dry run first to verify volume
    echo "[Data] Dry run..."
    python3 scripts/prepare_multibranch_data.py \
        --dry-run \
        --samples-per-type 1200 \
        --synthetic-columns 1200 \
        --ratio-distilled 0.3 \
        --seed 42

    echo ""
    echo "[Data] Full extraction..."
    python3 scripts/prepare_multibranch_data.py \
        --distilled output/distillation-v3/sherlock_distilled.csv.gz \
        --finetype ./target/release/finetype \
        --output "$FTMB_FILE" \
        --label-remap data/label_remap.json \
        --samples-per-type 1200 \
        --synthetic-columns 1200 \
        --ratio-distilled 0.3 \
        --seed 42 \
        --workers 8

    echo ""
    echo "[Data] Verifying output..."
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify

    echo ""
    echo "[Data] Complete: $(date)"
fi
echo ""

# ─── Step 2: Train Flat Multi-Branch (M-6) ───────────────────────

echo "════════════════════════════════════════════════════════════════"
echo " Step 2/4: Train Flat Multi-Branch (20 epochs)"
echo " Started: $(date)"
echo "════════════════════════════════════════════════════════════════"

FLAT_MODEL="models/sherlock-v2-flat"

if [[ -f "$FLAT_MODEL/model.safetensors" ]]; then
    echo "[Flat] Model already exists, skipping training"
else
    cargo run --bin finetype --no-default-features --features metal --release -- \
        train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$FLAT_MODEL" \
        --epochs 20 \
        --batch-size 32 \
        --lr 0.0001 \
        --weight-decay 0.0001 \
        --dropout 0.35 \
        --seed 42 \
        --head flat \
        --patience 10 \
        --no-tui \
    2>&1
fi

echo ""
echo "Flat training complete: $(date)"
echo ""

# ─── Step 3: Train Hierarchical Multi-Branch ─────────────────────

HIER_MODEL="models/sherlock-v2-hier"

if [[ "$FLAT_ONLY" == "false" ]]; then
    echo "════════════════════════════════════════════════════════════════"
    echo " Step 3/4: Train Hierarchical Multi-Branch (15 epochs)"
    echo " Started: $(date)"
    echo " Note: Trained unconditionally (diverges from M-7 conditionality)"
    echo "════════════════════════════════════════════════════════════════"

    if [[ -f "$HIER_MODEL/model.safetensors" ]]; then
        echo "[Hier] Model already exists, skipping training"
    else
        cargo run --bin finetype --no-default-features --features metal --release -- \
            train-multi-branch \
            --data "$FTMB_FILE" \
            --output "$HIER_MODEL" \
            --epochs 15 \
            --batch-size 32 \
            --lr 0.0001 \
            --weight-decay 0.0001 \
            --dropout 0.35 \
            --seed 42 \
            --head hierarchical \
            --patience 7 \
            --no-tui \
        2>&1
    fi

    echo ""
    echo "Hierarchical training complete: $(date)"
    echo ""
else
    echo "[Step 3/4] Hierarchical training — SKIPPED (--flat-only)"
    echo ""
fi

# ─── Step 4: Evaluation ──────────────────────────────────────────

echo "════════════════════════════════════════════════════════════════"
echo " Step 4/4: Evaluation"
echo " Started: $(date)"
echo "════════════════════════════════════════════════════════════════"
echo ""

# Only eval models that were trained in this run
MODELS_TO_EVAL=("$FLAT_MODEL")
[[ "$FLAT_ONLY" == "false" ]] && MODELS_TO_EVAL+=("$HIER_MODEL")

for model_dir in "${MODELS_TO_EVAL[@]}"; do
    model_name="$(basename "$model_dir")"
    if [[ -f "$model_dir/model.safetensors" ]]; then
        echo "── Evaluating $model_name ──"
        ./scripts/eval.sh --model "$model_dir" || {
            echo "WARN: Eval failed for $model_dir"
            continue
        }

        # Preserve eval results for this model (prevents overwriting)
        mkdir -p "$model_dir/eval"
        cp -r eval/eval_output/* "$model_dir/eval/" 2>/dev/null || true
        echo "  Eval results saved to $model_dir/eval/"
        echo ""
    else
        echo "── Skipping $model_name (no model.safetensors) ──"
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

# Print eval scores summary (reads JSON with DuckDB — works on macOS + Linux)
echo "Results:"
for model_dir in "${MODELS_TO_EVAL[@]}"; do
    model_name="$(basename "$model_dir")"
    JSON_FILE="$model_dir/eval/profile_results.json"
    if [[ -f "$JSON_FILE" ]]; then
        echo "  $model_name:"
        duckdb -c "SELECT
            '    Label accuracy: ' || label_correct || '/' || label_total || ' (' || label_accuracy_pct || '%)' AS result
          FROM read_json_auto('$JSON_FILE')
          UNION ALL
          SELECT
            '    Domain accuracy: ' || domain_correct || '/' || domain_total || ' (' || domain_accuracy_pct || '%)'
          FROM read_json_auto('$JSON_FILE')" -noheader -csv 2>/dev/null || echo "    (failed to read eval JSON)"
    elif [[ -f "$model_dir/eval/report.md" ]]; then
        echo "  $model_name: eval completed (no JSON summary — check eval version)"
    else
        echo "  $model_name: no eval results"
    fi
done
echo ""
echo "Next steps:"
echo "  1. Compare flat vs hierarchical accuracy"
echo "  2. Run trace analysis: specs/2026-03-23-multibranch-eval-diagnosis/trace-analysis.md"
echo "  3. Review findings before merging PR #21"
