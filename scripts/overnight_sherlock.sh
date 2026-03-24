#!/usr/bin/env bash
# scripts/overnight_sherlock.sh — Overnight multi-branch training pipeline (v4)
#
# Runs on M1 Pro with Metal acceleration. Expects:
#   - Distilled data at output/distillation-v3/sherlock_distilled.csv.gz
#   - Model2Vec at models/model2vec/
#   - Label remap at data/label_remap.json
#   - Sibling-context model at models/sibling-context/ (for enriched training)
#
# Pipeline:
#   1. Prepare 300k+ FTMB v3 training data (table-grouped, 50/50 mix)
#   2. Train flat model WITH frozen sibling-context enrichment (20 epochs)
#   3. Train flat model WITHOUT sibling enrichment (same v3 data, ablation baseline)
#   4. Evaluate both models against Tier 1 profile eval
#   5. Ablation comparison: sibling-enriched vs raw headers (AC-7)
#
# Usage:
#   ./scripts/overnight_sherlock.sh                  # Full pipeline
#   ./scripts/overnight_sherlock.sh --skip-data       # Skip data prep (reuse existing .ftmb)
#   ./scripts/overnight_sherlock.sh --skip-ablation   # Skip ablation variant
#
# Output:
#   output/multibranch-training/blend-50-50-v3.ftmb  — Training data (FTMB v3, table-grouped)
#   models/sherlock-v4-sibling/                       — Flat model with sibling enrichment
#   models/sherlock-v4-baseline/                      — Flat model without sibling enrichment
#   results/overnight-v4.log                          — Full log
#
# Ablation design (AC-7):
#   Both models train on identical FTMB v3 data with table groups.
#   "sibling" model: FrozenSiblingContext loads from models/sibling-context/,
#     enriching header embeddings via frozen attention per group.
#   "baseline" model: sibling-context model is temporarily hidden,
#     so FrozenSiblingContext doesn't load → flat batching with raw headers.
#   Delta measures sibling signal contribution.
#
# v3 config (preserved for reproducibility):
#   --format v2, blend-50-50.ftmb, sherlock-v3-{flat,hier,noheader}/
#   To reproduce v3: git checkout <v3-commit> -- scripts/overnight_sherlock.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v4.log"

SKIP_DATA=false
SKIP_ABLATION=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-data)      SKIP_DATA=true; shift ;;
        --skip-ablation)  SKIP_ABLATION=true; shift ;;
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
echo " Multi-Branch Overnight Pipeline v4 (Sibling Context)"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo " Config: FTMB v3, 50/50 distilled/synthetic, seed 42, 20 epochs"
echo "================================================================"
echo ""

PIPELINE_START=$(date +%s)

# --- Pre-flight checks ------------------------------------------------

echo "[Pre-flight] Checking prerequisites..."

if [[ ! -f output/distillation-v3/sherlock_distilled.csv.gz ]]; then
    echo "FAIL: Distilled data not found at output/distillation-v3/sherlock_distilled.csv.gz"
    exit 1
fi

if [[ ! -d models/model2vec ]]; then
    echo "FAIL: Model2Vec not found at models/model2vec/"
    exit 1
fi

SIBLING_CTX_DIR="models/sibling-context"
if [[ ! -f "$SIBLING_CTX_DIR/model.safetensors" ]]; then
    echo "FAIL: Sibling-context model not found at $SIBLING_CTX_DIR/model.safetensors"
    echo "  This is required for v4 pipeline. Train with:"
    echo "    cargo run --release -- train-sibling-context"
    exit 1
fi
echo "[Pre-flight] Sibling-context model found at $SIBLING_CTX_DIR/"

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

# --- Step 1: Prepare Training Data (FTMB v3, table-grouped) ----------

FTMB_FILE="output/multibranch-training/blend-50-50-v3.ftmb"
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1/4: Data prep — SKIPPED (--skip-data, reusing existing)"
    echo "================================================================"
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    echo ""
else
    echo "================================================================"
    echo " Step 1/4: Prepare 300k+ Training Data (FTMB v3, table-grouped)"
    echo " Started: $(date)"
    echo "================================================================"

    # Dry run first to verify volume
    echo "[Data] Dry run..."
    python3 scripts/prepare_multibranch_data.py \
        --dry-run \
        --samples-per-type 1200 \
        --synthetic-columns 1200 \
        --ratio-distilled 0.5 \
        --format v3 \
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
        --ratio-distilled 0.5 \
        --format v3 \
        --seed 42 \
        --workers 8

    echo ""
    echo "[Data] Verifying output..."
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify

    echo ""
    echo "[Data] Complete: $(date)"
fi
echo ""

# --- Step 2: Train WITH Sibling-Context Enrichment --------------------
#
# FrozenSiblingContext loads from models/sibling-context/ automatically.
# Training loop uses batch_groups() with frozen attention enrichment.

SIBLING_MODEL="models/sherlock-v4-sibling"

echo "================================================================"
echo " Step 2/4: Train Flat Model WITH Sibling-Context Enrichment"
echo " Started: $(date)"
echo " Sibling context: $SIBLING_CTX_DIR/"
echo "================================================================"

if [[ -f "$SIBLING_MODEL/model.safetensors" ]]; then
    echo "[Sibling] Model already exists, skipping training"
else
    cargo run --bin finetype --no-default-features --features metal --release -- \
        train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$SIBLING_MODEL" \
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
echo "Sibling-enriched training complete: $(date)"
echo ""

# --- Step 3: Train WITHOUT Sibling-Context (Ablation Baseline) --------
#
# Same v3 data, same hyperparameters, but sibling-context model is
# temporarily hidden so FrozenSiblingContext doesn't load. Training
# falls back to flat batching with raw header features.

BASELINE_MODEL="models/sherlock-v4-baseline"

if [[ "$SKIP_ABLATION" == "false" ]]; then
    echo "================================================================"
    echo " Step 3/4: Train Flat Model WITHOUT Sibling-Context (Ablation)"
    echo " Started: $(date)"
    echo " Purpose: AC-7 — baseline for sibling signal measurement"
    echo "================================================================"

    if [[ -f "$BASELINE_MODEL/model.safetensors" ]]; then
        echo "[Baseline] Model already exists, skipping training"
    else
        # Temporarily hide sibling-context model so it doesn't load
        SIBLING_HIDDEN="${SIBLING_CTX_DIR}.hidden"
        mv "$SIBLING_CTX_DIR" "$SIBLING_HIDDEN"
        trap 'mv "$SIBLING_HIDDEN" "$SIBLING_CTX_DIR" 2>/dev/null || true' EXIT

        cargo run --bin finetype --no-default-features --features metal --release -- \
            train-multi-branch \
            --data "$FTMB_FILE" \
            --output "$BASELINE_MODEL" \
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

        # Restore sibling-context model
        mv "$SIBLING_HIDDEN" "$SIBLING_CTX_DIR"
        trap - EXIT
    fi

    echo ""
    echo "Baseline training complete: $(date)"
    echo ""
else
    echo "[Step 3/4] Ablation baseline — SKIPPED (--skip-ablation)"
    echo ""
fi

# --- Step 4: Evaluation -----------------------------------------------

echo "================================================================"
echo " Step 4/4: Evaluation"
echo " Started: $(date)"
echo "================================================================"
echo ""

MODELS_TO_EVAL=("$SIBLING_MODEL")
[[ "$SKIP_ABLATION" == "false" ]] && MODELS_TO_EVAL+=("$BASELINE_MODEL")

for model_dir in "${MODELS_TO_EVAL[@]}"; do
    model_name="$(basename "$model_dir")"
    if [[ -f "$model_dir/model.safetensors" ]]; then
        echo "-- Evaluating $model_name --"
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
        echo "-- Skipping $model_name (no model.safetensors) --"
        echo ""
    fi
done

# --- Ablation Comparison (AC-7) ----------------------------------------

if [[ "$SKIP_ABLATION" == "false" ]]; then
    SIBLING_JSON="$SIBLING_MODEL/eval/profile_results.json"
    BASELINE_JSON="$BASELINE_MODEL/eval/profile_results.json"

    if [[ -f "$SIBLING_JSON" ]] && [[ -f "$BASELINE_JSON" ]]; then
        echo "================================================================"
        echo " Sibling-Context Ablation Comparison (AC-7)"
        echo "================================================================"
        echo ""
        echo " Both models trained on identical FTMB v3 data."
        echo " Sibling: FrozenSiblingContext enriches headers via frozen attention."
        echo " Baseline: raw Model2Vec headers, no sibling enrichment."
        echo ""
        duckdb -c "
            WITH sibling AS (
                SELECT label_correct, label_total, label_accuracy_pct,
                       domain_correct, domain_total, domain_accuracy_pct
                FROM read_json_auto('$SIBLING_JSON')
            ),
            baseline AS (
                SELECT label_correct, label_total, label_accuracy_pct,
                       domain_correct, domain_total, domain_accuracy_pct
                FROM read_json_auto('$BASELINE_JSON')
            )
            SELECT
                'With siblings:    ' || sibling.label_correct || '/' || sibling.label_total
                    || ' (' || sibling.label_accuracy_pct || '% label), '
                    || sibling.domain_correct || '/' || sibling.domain_total
                    || ' (' || sibling.domain_accuracy_pct || '% domain)' AS result
            FROM sibling
            UNION ALL
            SELECT
                'Without siblings: ' || baseline.label_correct || '/' || baseline.label_total
                    || ' (' || baseline.label_accuracy_pct || '% label), '
                    || baseline.domain_correct || '/' || baseline.domain_total
                    || ' (' || baseline.domain_accuracy_pct || '% domain)'
            FROM baseline
            UNION ALL
            SELECT
                'Delta:            ' || (sibling.label_accuracy_pct - baseline.label_accuracy_pct)
                    || ' pp label, '
                    || (sibling.domain_accuracy_pct - baseline.domain_accuracy_pct)
                    || ' pp domain'
            FROM sibling, baseline
        " -noheader -csv 2>/dev/null || echo "  (failed to compute ablation delta)"
        echo ""
    fi
fi

# --- Summary -----------------------------------------------------------

PIPELINE_END=$(date +%s)
TOTAL_ELAPSED=$((PIPELINE_END - PIPELINE_START))
TOTAL_HOURS=$((TOTAL_ELAPSED / 3600))
TOTAL_MINS=$(( (TOTAL_ELAPSED % 3600) / 60))

echo "================================================================"
echo " Pipeline Complete"
echo " Finished: $(date)"
echo " Total time: ${TOTAL_HOURS}h ${TOTAL_MINS}m"
echo " Log: $LOG_FILE"
echo "================================================================"
echo ""

# Print eval scores summary (reads JSON with DuckDB)
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
echo "  1. Compare sibling-enriched vs baseline accuracy"
echo "  2. If delta > 0: sibling context improves accuracy, proceed to pipeline integration"
echo "  3. If delta ≤ 0: see fallback plan in spec (retrain attention against multi-branch loss)"
echo "  4. Review findings before merging"
