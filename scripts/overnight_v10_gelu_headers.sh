#!/usr/bin/env bash
# scripts/overnight_v10_gelu_headers.sh — GELU+LN with proper header/sibling data
#
# Fair comparison of GELU+LN architecture against v4-sibling baseline.
#
# v8/v9 trained on v7-blend-50-50.ftmb which had ALL-ZERO header features
# and empty sibling strings — the header branch learned nothing. v4-sibling
# trained on v3 data with real Model2Vec headers + frozen sibling-context
# enrichment. The comparison was invalid: 4-branch vs effectively 3-branch.
#
# This script generates FRESH training data with proper headers and validates
# header features are non-zero before training begins.
#
# What's different from v9:
#   - Fresh FTMB file (v10-blend-50-50.ftmb) — never reuses v7 data
#   - Header validation: spot-checks FTMB records for non-zero headers
#   - Hard-fail if Model2Vec or sibling-context model is missing
#   - Same GELU+LN architecture, same conservative hyperparameters
#
# Expected outcome:
#   This is the first fair comparison of GELU+LN vs ReLU+BN with equivalent
#   header/sibling data. If GELU+LN is truly better (higher val_accuracy in
#   training), it should now translate to profile eval improvement.
#
# Runs on M1 Pro with Metal acceleration. Requires:
#   - Distilled data at output/distillation-v3/sherlock_distilled.csv.gz
#   - Model2Vec at models/model2vec/ (HARD REQUIREMENT)
#   - Sibling-context model at models/sibling-context/ (HARD REQUIREMENT)
#   - Label remap at data/label_remap.json
#
# Usage:
#   ./scripts/overnight_v10_gelu_headers.sh                 # Full pipeline
#   ./scripts/overnight_v10_gelu_headers.sh --skip-data     # Skip data prep
#   ./scripts/overnight_v10_gelu_headers.sh --epochs N      # Override epoch count
#
# Output:
#   output/multibranch-training/v10-blend-50-50.ftmb           — Training data (fresh)
#   models/sherlock-v10-gelu/                                   — Trained model
#   results/overnight-v10-gelu-headers.log                      — Full log
#   results/eval-pack-sherlock-v10-gelu.tar.gz                  — Eval pack for transfer
#
# Spec: specs/2026-04-11-candle-autoresearch-port/spec.yaml
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v10-gelu-headers.log"

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
echo " Multi-Branch Overnight Pipeline v10 — GELU+LN with Headers"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo " Config: FTMB v3, 50/50 distilled/synthetic, seed 42"
echo "         $EPOCHS epochs, GELU activation, LayerNorm everywhere"
echo "         lr=0.0001, weight_decay=0.0001"
echo ""
echo " Purpose: Fair GELU+LN comparison — v8/v9 trained on data"
echo "   with all-zero headers. This generates fresh data with real"
echo "   Model2Vec header features and sibling-context enrichment."
echo ""
echo " Spec: specs/2026-04-11-candle-autoresearch-port/"
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

# Model2Vec is REQUIRED — headers are the whole point of v10
if [[ ! -d models/model2vec ]] || [[ ! -f models/model2vec/model.safetensors ]]; then
    echo "FAIL: Model2Vec not found at models/model2vec/"
    echo "  Header features require Model2Vec. This is a hard requirement for v10."
    exit 1
fi
echo "[Pre-flight] Model2Vec: OK (models/model2vec/)"

# Sibling-context is REQUIRED — enrichment is the whole point of v10
SIBLING_CTX_DIR="models/sibling-context"
if [[ ! -f "$SIBLING_CTX_DIR/model.safetensors" ]]; then
    echo "FAIL: Sibling-context model not found at $SIBLING_CTX_DIR/model.safetensors"
    echo "  Frozen sibling-context enrichment is a hard requirement for v10."
    echo "  Train it with: cargo run --release -- train-sibling-context"
    exit 1
fi
echo "[Pre-flight] Sibling-context: OK ($SIBLING_CTX_DIR/)"

MODEL_CONFIG="models/sherlock-v6-gelu-config.json"
if [[ ! -f "$MODEL_CONFIG" ]]; then
    echo "FAIL: Model config not found at $MODEL_CONFIG"
    exit 1
fi

echo "[Pre-flight] Model config:"
cat "$MODEL_CONFIG"
echo ""

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

# Verify extract-features returns non-zero header features
echo "[Pre-flight] Verifying header feature extraction..."
HEADER_CHECK=$(echo '["hello world", "test value", "example"]' | \
    ./target/release/finetype extract-features --json --header "city_name" 2>/dev/null)
HEADER_NONZERO=$(echo "$HEADER_CHECK" | python3 -c "
import json, sys
data = json.load(sys.stdin)
hf = data.get('header_features', [])
nonzero = sum(1 for x in hf if abs(x) > 1e-6)
print(nonzero)
" 2>/dev/null || echo "0")

if [[ "$HEADER_NONZERO" -lt 10 ]]; then
    echo "FAIL: extract-features returned near-zero header features ($HEADER_NONZERO/128 nonzero)"
    echo "  Model2Vec may not be loading correctly during feature extraction."
    echo "  Check: models/model2vec/model.safetensors exists and is valid."
    exit 1
fi
echo "[Pre-flight] Header extraction OK ($HEADER_NONZERO/128 nonzero dimensions)"
echo ""

# --- Step 1: Prepare Training Data ------------------------------------

FTMB_FILE="output/multibranch-training/v10-blend-50-50.ftmb"
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1/4: Data prep — SKIPPED (--skip-data, reusing v10 FTMB)"
    echo "================================================================"
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    echo ""
else
    # Remove stale file if it exists — v10 always regenerates
    if [[ -f "$FTMB_FILE" ]] && [[ "$SKIP_DATA" != "true" ]]; then
        echo "[Data] Removing stale $FTMB_FILE to force regeneration..."
        rm -f "$FTMB_FILE"
    fi

    echo "================================================================"
    echo " Step 1/4: Prepare Training Data (FTMB v3, 50/50, 35% aug)"
    echo " Started: $(date)"
    echo " Output: $FTMB_FILE (fresh — not reusing v7)"
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

# --- Step 1.5: Validate header features in FTMB -------------------------

echo "================================================================"
echo " Step 1.5/4: Validate FTMB header features"
echo "================================================================"

# Use read_ftmb.py to dump first N records and check header features
HEADER_VALIDATION=$(python3 -c "
import struct, sys

path = '$FTMB_FILE'
with open(path, 'rb') as f:
    magic = f.read(4)
    version = struct.unpack('<H', f.read(2))[0]
    n_records = struct.unpack('<I', f.read(4))[0]
    char_dim = struct.unpack('<H', f.read(2))[0]
    embed_dim = struct.unpack('<H', f.read(2))[0]
    stats_dim = struct.unpack('<H', f.read(2))[0]
    header_dim = struct.unpack('<H', f.read(2))[0]

    if version < 3:
        print(f'FAIL: FTMB version {version} does not support table groups')
        sys.exit(1)

    n_groups = struct.unpack('<I', f.read(4))[0]

    # Check first 5 groups
    headers_checked = 0
    headers_nonzero = 0
    headers_with_name = 0

    for g in range(min(5, n_groups)):
        n_columns = struct.unpack('<H', f.read(2))[0]
        n_sibling_headers = struct.unpack('<H', f.read(2))[0]

        sibling_names = []
        for _ in range(n_sibling_headers):
            name_len = struct.unpack('<H', f.read(2))[0]
            name = f.read(name_len).decode('utf-8')
            sibling_names.append(name)
            if name.strip():
                headers_with_name += 1

        for c in range(n_columns):
            label_len = struct.unpack('<H', f.read(2))[0]
            label = f.read(label_len).decode('utf-8')
            col_idx = struct.unpack('<H', f.read(2))[0]
            char_feat = struct.unpack(f'<{char_dim}f', f.read(char_dim * 4))
            embed_feat = struct.unpack(f'<{embed_dim}f', f.read(embed_dim * 4))
            stats_feat = struct.unpack(f'<{stats_dim}f', f.read(stats_dim * 4))
            header_feat = struct.unpack(f'<{header_dim}f', f.read(header_dim * 4))

            headers_checked += 1
            nz = sum(1 for x in header_feat if abs(x) > 1e-6)
            if nz > 10:
                headers_nonzero += 1

    print(f'Groups: {n_groups}, Records: {n_records}')
    print(f'Sibling header names (first 5 groups): {headers_with_name} non-empty')
    print(f'Header features (first 5 groups): {headers_nonzero}/{headers_checked} with >10 nonzero dims')

    if headers_nonzero == 0:
        print('FAIL: All header features are zeros — Model2Vec not working during data prep')
        sys.exit(1)
    elif headers_nonzero < headers_checked * 0.5:
        print(f'WARN: Only {headers_nonzero}/{headers_checked} headers are non-zero')
    else:
        print('OK: Header features are populated')
" 2>&1)

echo "$HEADER_VALIDATION"

if echo "$HEADER_VALIDATION" | grep -q "^FAIL:"; then
    echo ""
    echo "Header validation failed. Aborting training."
    echo "Check that models/model2vec/ is accessible during data prep."
    exit 1
fi
echo ""

# --- Step 2: Train Model -----------------------------------------------

MODEL_DIR="models/sherlock-v10-gelu"

echo "================================================================"
echo " Step 2/4: Train GELU+LN Model (with real headers)"
echo " Started: $(date)"
echo " Config: $MODEL_CONFIG"
echo " Epochs: $EPOCHS, lr=0.0001, weight_decay=0.0001"
echo " Sibling context: $SIBLING_CTX_DIR/"
echo " Data: $FTMB_FILE (fresh v10, with headers)"
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
echo " Step 3/4: Evaluation"
echo " Started: $(date)"
echo "================================================================"
echo ""

if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
    echo "-- Evaluating sherlock-v10-gelu --"
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

# --- Step 4: Pack evaluation artifacts --------------------------------

echo "================================================================"
echo " Step 4/4: Packing evaluation artifacts"
echo " Started: $(date)"
echo "================================================================"
echo ""

./scripts/eval_pack.sh "$MODEL_DIR"
echo ""

# --- Also evaluate v4-sibling for direct comparison -------------------
# (Only if the model exists and we haven't already evaluated it recently)

V4_MODEL="models/sherlock-v4-sibling"
if [[ -f "$V4_MODEL/model.safetensors" ]]; then
    echo "================================================================"
    echo " Bonus: Re-evaluating v4-sibling baseline for direct comparison"
    echo " Started: $(date)"
    echo "================================================================"
    echo ""

    ./scripts/eval.sh --model "$V4_MODEL" || {
        echo "WARN: Eval failed for v4-sibling baseline"
    }

    mkdir -p "$V4_MODEL/eval"
    cp -r eval/eval_output/* "$V4_MODEL/eval/" 2>/dev/null || true
    ./scripts/eval_pack.sh "$V4_MODEL"
    echo ""
fi

# --- Comparison -------------------------------------------------------

V4_JSON="$V4_MODEL/eval/profile_results.json"
V10_JSON="$MODEL_DIR/eval/profile_results.json"

if [[ -f "$V4_JSON" ]] && [[ -f "$V10_JSON" ]] && command -v duckdb >/dev/null 2>&1; then
    echo "================================================================"
    echo " v4-sibling (ReLU+BN) vs v10-gelu (GELU+LN) — Fair Comparison"
    echo "================================================================"
    echo ""
    echo " Both models trained with:"
    echo "   - 4 branches: char + embed + stats + header (real Model2Vec)"
    echo "   - Frozen sibling-context enrichment during training"
    echo "   - FTMB v3 table-grouped data, 50/50 distilled/synthetic"
    echo "   - lr=0.0001, weight_decay=0.0001"
    echo ""
    duckdb -c "
        WITH v4 AS (SELECT * FROM read_json_auto('$V4_JSON')),
             v10 AS (SELECT * FROM read_json_auto('$V10_JSON'))
        SELECT
            'v4-sibling (ReLU+BN)' AS model,
            v4.label_correct AS label_correct,
            v4.label_total AS label_total,
            ROUND(v4.label_accuracy_pct, 1) AS label_pct,
            v4.domain_correct AS domain_correct,
            ROUND(v4.domain_accuracy_pct, 1) AS domain_pct
        FROM v4
        UNION ALL
        SELECT
            'v10-gelu (GELU+LN)' AS model,
            v10.label_correct,
            v10.label_total,
            ROUND(v10.label_accuracy_pct, 1) AS label_pct,
            v10.domain_correct,
            ROUND(v10.domain_accuracy_pct, 1) AS domain_pct
        FROM v10
        UNION ALL
        SELECT
            'DELTA (v10 - v4)',
            v10.label_correct - v4.label_correct,
            0,
            ROUND(v10.label_accuracy_pct - v4.label_accuracy_pct, 1),
            v10.domain_correct - v4.domain_correct,
            ROUND(v10.domain_accuracy_pct - v4.domain_accuracy_pct, 1)
        FROM v4, v10;
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
echo " Config: $MODEL_CONFIG (GELU+LN, scaled hidden dims)"
echo " Hyperparameters: lr=0.0001, weight_decay=0.0001"
echo " Log: $LOG_FILE"
echo " Eval pack: results/eval-pack-$(basename "$MODEL_DIR").tar.gz"
echo ""
echo " Key difference from v8/v9:"
echo "   v8/v9: trained on v7 FTMB with ALL-ZERO headers (dead header branch)"
echo "   v10:   trained on fresh FTMB with real Model2Vec headers + sibling ctx"
echo ""
echo " This is the first fair GELU+LN vs ReLU+BN comparison."
echo " Compare: v4-sibling baseline vs v10-gelu."
echo "================================================================"
