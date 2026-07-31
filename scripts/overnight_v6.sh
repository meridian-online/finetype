#!/usr/bin/env bash
# scripts/overnight_v6.sh — Overnight multi-branch training pipeline (v6)
#
# Runs on M1 Pro with Metal acceleration. Expects:
#   - Distilled data at output/distillation-v3/sherlock_distilled.csv.gz
#   - Model2Vec at models/model2vec/
#   - Label remap at data/label_remap.json
#   - Sibling-context model at models/sibling-context/
#
# v6 improvements over v4:
#   - Fixed generators: no 2-level HS codes (0043), wider calver (0044)
#   - Fixed augmentation: effective rate = configured rate (AC-5)
#   - Fixed synthetic pool: oversampled types get multiplier× more data (AC-6)
#   - Expanded eval: 154 types covered (AC-4), up from 129
#   - 9 stale label references fixed (AC-1)
#
# Pipeline:
#   1. Build with Metal
#   2. Prepare FTMB v3 training data (50/50 mix, 35% augmentation)
#   3. Train flat model with sibling-context enrichment (20 epochs)
#   4. Evaluate against expanded eval set
#   5. Compare with v5 baseline
#
# Usage:
#   ./scripts/overnight_v6.sh                 # Full pipeline
#   ./scripts/overnight_v6.sh --skip-data     # Skip data prep (reuse existing)
#   ./scripts/overnight_v6.sh --epochs N      # Override epoch count
#
# Output:
#   output/multibranch-training/v6-blend-50-50.ftmb  — Training data
#   models/sherlock-v6/                               — Trained model
#   results/overnight-v6.log                          — Full log
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v6.log"

SKIP_DATA=false
EPOCHS=20

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-data)  SKIP_DATA=true; shift ;;
        --epochs)     EPOCHS="$2"; shift 2 ;;
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
echo " Multi-Branch Overnight Pipeline v6 (Data Quality)"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo " Config: FTMB v3, 50/50 distilled/synthetic, seed 42"
echo "         $EPOCHS epochs, 35% augmentation rate"
echo " Spec: 2026-03-26-overnight-v6-data-quality"
echo "================================================================"
echo ""
echo " v6 changes:"
echo "   - Generator: hs_code 3+ levels only (MADR 0043)"
echo "   - Generator: calver 4 format variants (MADR 0044)"
echo "   - Augmentation: effective rate = configured (AC-5)"
echo "   - Synthetic pool: scales for oversampled types (AC-6)"
echo "   - Eval: 154 types covered (AC-4)"
echo "   - Stale labels: 9 fixed across pipeline (AC-1)"
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
    exit 1
fi
echo "[Pre-flight] Sibling-context model found at $SIBLING_CTX_DIR/"

if [[ ! -f data/label_remap.json ]]; then
    echo "WARN: Label remap not found at data/label_remap.json (proceeding without remap)"
fi

if ! command -v duckdb >/dev/null 2>&1; then
    echo "FAIL: duckdb CLI not found on PATH"
    exit 1
fi

# Verify taxonomy/generator alignment (post-AC-2 fixes)
echo "[Pre-flight] Checking taxonomy alignment..."
cargo run --bin finetype --no-default-features --features metal --release -- check 2>&1 | tail -3
echo ""

echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"
echo ""

# --- Step 1: Prepare Training Data ------------------------------------

FTMB_FILE="output/multibranch-training/v6-blend-50-50.ftmb"
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1/3: Data prep — SKIPPED (--skip-data, reusing existing)"
    echo "================================================================"
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    echo ""
else
    echo "================================================================"
    echo " Step 1/3: Prepare Training Data (FTMB v3, 50/50, 35% aug)"
    echo " Started: $(date)"
    echo "================================================================"

    # Dry run first
    echo "[Data] Dry run..."
    python3 scripts/prepare_multibranch_data.py \
        --dry-run \
        --samples-per-type 1200 \
        --synthetic-columns 1200 \
        --ratio-distilled 0.5 \
        --augmentation-rate 0.35 \
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
        --augmentation-rate 0.35 \
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

# --- Step 2: Train Model -----------------------------------------------

MODEL_DIR="models/sherlock-v6"

echo "================================================================"
echo " Step 2/3: Train Flat Model (sherlock-v6)"
echo " Started: $(date)"
echo " Sibling context: $SIBLING_CTX_DIR/"
echo " Epochs: $EPOCHS"
echo "================================================================"

if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
    echo "[Train] Model already exists at $MODEL_DIR, skipping training"
else
    cargo run --bin finetype --no-default-features --features metal --release -- \
        train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$MODEL_DIR" \
        --epochs "$EPOCHS" \
        --batch-size 32 \
        --lr 0.0001 \
        --weight-decay 0.0001 \
        --dropout 0.35 \
        --seed 42 \
        --head flat \
        --patience 10 \
        \
    2>&1
fi

echo ""
echo "Training complete: $(date)"
echo ""

# --- Step 3: Evaluation -----------------------------------------------

echo "================================================================"
echo " Step 3/3: Evaluation"
echo " Started: $(date)"
echo "================================================================"
echo ""

if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
    echo "-- Evaluating sherlock-v6 --"
    ./scripts/eval.sh --model "$MODEL_DIR" || {
        echo "WARN: Eval failed for $MODEL_DIR"
    }

    # Preserve eval results
    mkdir -p "$MODEL_DIR/eval"
    cp -r eval/eval_output/* "$MODEL_DIR/eval/" 2>/dev/null || true
    echo "  Eval results saved to $MODEL_DIR/eval/"
    echo ""
else
    echo "FAIL: No trained model at $MODEL_DIR"
    exit 1
fi

# --- Comparison with v5 baseline ----------------------------------------

V5_JSON="models/sherlock-v5-current/eval/profile_results.json"
V6_JSON="$MODEL_DIR/eval/profile_results.json"

if [[ -f "$V5_JSON" ]] && [[ -f "$V6_JSON" ]]; then
    echo "================================================================"
    echo " v5 → v6 Comparison"
    echo "================================================================"
    echo ""
    duckdb -c "
        WITH v5 AS (SELECT * FROM read_json_auto('$V5_JSON')),
             v6 AS (SELECT * FROM read_json_auto('$V6_JSON'))
        SELECT
            'v5 (baseline)' AS model,
            v5.label_correct AS label_correct,
            v5.label_total AS label_total,
            ROUND(v5.label_accuracy_pct, 1) AS label_pct,
            v5.domain_correct AS domain_correct,
            ROUND(v5.domain_accuracy_pct, 1) AS domain_pct
        FROM v5
        UNION ALL
        SELECT
            'v6 (data quality)' AS model,
            v6.label_correct,
            v6.label_total,
            ROUND(v6.label_accuracy_pct, 1) AS label_pct,
            v6.domain_correct,
            ROUND(v6.domain_accuracy_pct, 1) AS domain_pct
        FROM v6
        UNION ALL
        SELECT
            'DELTA',
            v6.label_correct - v5.label_correct,
            0,
            ROUND(v6.label_accuracy_pct - v5.label_accuracy_pct, 1),
            v6.domain_correct - v5.domain_correct,
            ROUND(v6.domain_accuracy_pct - v5.domain_accuracy_pct, 1)
        FROM v5, v6;
    " 2>/dev/null || echo "(comparison query failed — check JSON structure)"
    echo ""
fi

# --- Summary ──────────────────────────────────────────────────────────

PIPELINE_END=$(date +%s)
ELAPSED=$(( PIPELINE_END - PIPELINE_START ))
HOURS=$(( ELAPSED / 3600 ))
MINUTES=$(( (ELAPSED % 3600) / 60 ))

echo "================================================================"
echo " Pipeline Complete"
echo " Finished: $(date)"
echo " Duration: ${HOURS}h ${MINUTES}m"
echo " Model: $MODEL_DIR"
echo " Log: $LOG_FILE"
echo "================================================================"
