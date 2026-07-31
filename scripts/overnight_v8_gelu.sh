#!/usr/bin/env bash
# scripts/overnight_v8_gelu.sh — Overnight multi-branch training with GELU+LayerNorm
#
# Ports autoresearch findings to Candle: GELU activations, LayerNorm everywhere,
# weight_decay=0.01, lr=0.001, production-scale dimensions.
#
# Runs on M1 Pro with Metal acceleration. Expects:
#   - Distilled data at output/distillation-v3/sherlock_distilled.csv.gz
#   - Model2Vec at models/model2vec/
#   - Label remap at data/label_remap.json
#   - Sibling-context model at models/sibling-context/
#
# Key changes over v7:
#   - Activation: GELU (was ReLU) — autoresearch +1.3pp
#   - Normalization: LayerNorm on all branches + merge (was BatchNorm on merge only)
#   - Learning rate: 0.001 (was 0.0001) — 10x from autoresearch winner
#   - Weight decay: 0.01 (was 0.0001)
#   - Model config: models/sherlock-v6-gelu-config.json
#
# Usage:
#   ./scripts/overnight_v8_gelu.sh                 # Full pipeline
#   ./scripts/overnight_v8_gelu.sh --skip-data     # Skip data prep (reuse existing)
#   ./scripts/overnight_v8_gelu.sh --epochs N      # Override epoch count
#
# Output:
#   output/multibranch-training/v7-blend-50-50.ftmb  — Training data (reused if exists)
#   models/sherlock-v6-gelu/                          — Trained model
#   results/overnight-v8-gelu.log                     — Full log
#
# Spec: 2026-04-11-candle-autoresearch-port
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v8-gelu.log"

SKIP_DATA=false
EPOCHS=30

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
echo " Multi-Branch Overnight Pipeline v8 — GELU+LayerNorm Port"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo " Config: FTMB v3, 50/50 distilled/synthetic, seed 42"
echo "         $EPOCHS epochs, GELU activation, LayerNorm everywhere"
echo "         lr=0.001, weight_decay=0.01 (autoresearch winner)"
echo " Spec: 2026-04-11-candle-autoresearch-port"
echo "================================================================"
echo ""

PIPELINE_START=$(date +%s)

# --- Pre-flight checks ------------------------------------------------

echo "[Pre-flight] Checking prerequisites..."

DISTILLED_FILE="output/distillation-v3/sherlock_distilled.csv.gz"
HF_DATASET="meridian-online/sherlock-annotated"

if [[ ! -f "$DISTILLED_FILE" ]]; then
    echo "[Pre-flight] Distilled data not found locally. Downloading from HuggingFace..."
    mkdir -p output/distillation-v3

    # Download from HuggingFace and convert Parquet → CSV.gz
    # The dataset has: sherlock_index, split, sample_values, blind_label,
    # blind_confidence, finetype_label, finetype_confidence, agreement,
    # final_label, reasoning, ground_truth_label

    # Prefer uv for dependency-free execution, fall back to pip
    if command -v uv >/dev/null 2>&1; then
        echo "[Pre-flight] Using uv to run download (auto-installs datasets)..."
        uv run --with datasets python3 -c "
from datasets import load_dataset
import csv, gzip

print('Downloading $HF_DATASET from HuggingFace...')
ds = load_dataset('$HF_DATASET', split='train')
print(f'Downloaded {len(ds)} rows')

print('Writing to $DISTILLED_FILE...')
with gzip.open('$DISTILLED_FILE', 'wt', newline='') as f:
    writer = csv.DictWriter(f, fieldnames=ds.column_names)
    writer.writeheader()
    for row in ds:
        writer.writerow(row)

print(f'Saved {len(ds)} rows to $DISTILLED_FILE')
" 2>&1
    else
        echo "[Pre-flight] uv not found, trying python3 directly..."
        python3 -c "
from datasets import load_dataset
import csv, gzip

print('Downloading $HF_DATASET from HuggingFace...')
ds = load_dataset('$HF_DATASET', split='train')
print(f'Downloaded {len(ds)} rows')

print('Writing to $DISTILLED_FILE...')
with gzip.open('$DISTILLED_FILE', 'wt', newline='') as f:
    writer = csv.DictWriter(f, fieldnames=ds.column_names)
    writer.writeheader()
    for row in ds:
        writer.writerow(row)

print(f'Saved {len(ds)} rows to $DISTILLED_FILE')
" 2>&1
    fi

    if [[ ! -f "$DISTILLED_FILE" ]]; then
        echo "FAIL: Download failed. Install uv (https://docs.astral.sh/uv/) or run: pip3 install datasets"
        exit 1
    fi
    echo "[Pre-flight] Download complete: $(ls -lh "$DISTILLED_FILE" | awk '{print $5}')"
fi

if [[ ! -d models/model2vec ]]; then
    echo "FAIL: Model2Vec not found at models/model2vec/"
    exit 1
fi

MODEL_CONFIG="models/sherlock-v6-gelu-config.json"
if [[ ! -f "$MODEL_CONFIG" ]]; then
    echo "FAIL: Model config not found at $MODEL_CONFIG"
    exit 1
fi

echo "[Pre-flight] Model config:"
cat "$MODEL_CONFIG"
echo ""

SIBLING_CTX_DIR="models/sibling-context"
if [[ ! -f "$SIBLING_CTX_DIR/model.safetensors" ]]; then
    echo "WARN: Sibling-context model not found at $SIBLING_CTX_DIR (proceeding without)"
fi

if [[ ! -f data/label_remap.json ]]; then
    echo "WARN: Label remap not found at data/label_remap.json (proceeding without remap)"
fi

if ! command -v duckdb >/dev/null 2>&1; then
    echo "WARN: duckdb CLI not found on PATH (summary queries will be skipped)"
fi

# Build with Metal
echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"
echo ""

# Verify GELU+LN fields in the binary
echo "[Pre-flight] Verifying model config deserialization..."
cargo test -p finetype-train -- test_config_backward_compat_deserializes_without_new_fields --quiet 2>&1
cargo test -p finetype-train -- test_forward_pass_shape_gelu_layer_norm --quiet 2>&1
echo "[Pre-flight] Config + forward pass tests OK"
echo ""

# --- Step 1: Prepare Training Data ------------------------------------

FTMB_FILE="output/multibranch-training/v7-blend-50-50.ftmb"
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1/3: Data prep — SKIPPED (--skip-data, reusing existing)"
    echo "================================================================"
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    echo ""
elif [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1/3: Data prep — Reusing existing FTMB"
    echo "================================================================"
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    echo ""
else
    echo "================================================================"
    echo " Step 1/3: Prepare Training Data (FTMB v3, 50/50, 35% aug)"
    echo " Started: $(date)"
    echo "================================================================"

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

MODEL_DIR="models/sherlock-v6-gelu"

echo "================================================================"
echo " Step 2/3: Train GELU+LN Model (sherlock-v6-gelu)"
echo " Started: $(date)"
echo " Config: $MODEL_CONFIG"
echo " Epochs: $EPOCHS, lr=0.001, weight_decay=0.01"
echo "================================================================"

if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
    echo "[Train] Model already exists at $MODEL_DIR, skipping training"
else
    cargo run --bin finetype --no-default-features --features metal --release -- \
        train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$MODEL_DIR" \
        --model-config "$MODEL_CONFIG" \
        --epochs "$EPOCHS" \
        --batch-size 32 \
        --lr 0.001 \
        --weight-decay 0.01 \
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

# --- Verify saved config contains GELU+LN --------------------------------

SAVED_CONFIG="$MODEL_DIR/config.json"
if [[ -f "$SAVED_CONFIG" ]]; then
    echo "[Verify] Checking saved config.json for GELU+LN fields..."
    if grep -q '"GELU"' "$SAVED_CONFIG" && grep -q '"use_layer_norm": true' "$SAVED_CONFIG"; then
        echo "[Verify] OK: config.json contains activation=GELU and use_layer_norm=true"
    else
        echo "[Verify] WARNING: config.json may not contain expected GELU+LN fields!"
        echo "  Contents:"
        cat "$SAVED_CONFIG"
    fi
    echo ""
fi

# --- Training Summary (from results.json) --------------------------------

RESULTS_JSON="$MODEL_DIR/results.json"
if [[ -f "$RESULTS_JSON" ]] && command -v duckdb >/dev/null 2>&1; then
    echo "================================================================"
    echo " Training Summary (from $RESULTS_JSON)"
    echo "================================================================"
    echo ""
    duckdb -c "
        WITH metrics AS (
            SELECT unnest(from_json_strict(
                content::JSON, '[{\"epoch\":0,\"train_loss\":0.0,\"val_loss\":0.0,\"train_accuracy\":0.0,\"val_accuracy\":0.0,\"learning_rate\":0.0,\"epoch_time_secs\":0.0}]'
            )) AS m
            FROM read_text('$RESULTS_JSON')
        ),
        flat AS (
            SELECT m.epoch, m.train_loss, m.val_loss,
                   m.train_accuracy, m.val_accuracy,
                   m.learning_rate, m.epoch_time_secs
            FROM metrics
        ),
        best AS (
            SELECT * FROM flat ORDER BY val_accuracy DESC LIMIT 1
        )
        SELECT
            (SELECT count(*) FROM flat) AS total_epochs,
            ROUND(best.val_accuracy * 100, 1) AS best_val_acc_pct,
            best.epoch + 1 AS best_epoch,
            ROUND(best.val_loss, 4) AS best_val_loss,
            ROUND(best.train_loss, 4) AS best_train_loss,
            printf('%.2e', best.learning_rate) AS lr_at_best,
            ROUND((SELECT sum(epoch_time_secs) FROM flat) / 60, 1) AS total_train_min
        FROM best;
    " 2>/dev/null || echo "(summary query failed — check results.json structure)"
    echo ""
fi

# --- Step 3: Evaluation -----------------------------------------------

echo "================================================================"
echo " Step 3/3: Evaluation"
echo " Started: $(date)"
echo "================================================================"
echo ""

if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
    echo "-- Evaluating sherlock-v6-gelu --"
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
echo " Config: $MODEL_CONFIG"
echo " Log: $LOG_FILE"
echo ""
echo " Exit condition check:"
echo "   >= 160/190 → Publish to HuggingFace"
echo "   155-159    → Keep locally, iterate"
echo "   < 155      → Revert, investigate"
echo "================================================================"
