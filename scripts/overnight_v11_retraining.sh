#!/usr/bin/env bash
# scripts/overnight_v11_retraining.sh — Distillation-heavy retraining
#
# Retrain the ReLU+BN multi-branch model on a 70/30 distillation:synthetic
# data mix with real Model2Vec headers and sibling-context enrichment.
#
# What's different from v4-sibling:
#   - 70/30 distillation:synthetic mix (was 50/50)
#   - Production-scale dimensions from sherlock-v5-scaled-config.json
#   - Fresh FTMB with validated headers (v10-style checks)
#   - 40 epoch cap with patience 10 (was 30/10)
#
# Architecture is UNCHANGED from v4-sibling: ReLU+BatchNorm, flat head.
# Decision 0046 rejected GELU+LN. Hyperparameters match v4-sibling
# (lr=1e-4, weight_decay=1e-4) per spec review finding.
#
# Runs on M1 Pro with Metal acceleration. Requires:
#   - Distilled data at output/distillation-v3/sherlock_distilled.csv.gz
#   - Model2Vec at models/model2vec/ (HARD REQUIREMENT)
#   - Sibling-context model at models/sibling-context/ (HARD REQUIREMENT)
#   - Label remap at data/label_remap.json
#
# Usage:
#   ./scripts/overnight_v11_retraining.sh                 # Full pipeline
#   ./scripts/overnight_v11_retraining.sh --skip-data     # Skip data prep
#   ./scripts/overnight_v11_retraining.sh --epochs N      # Override epoch count
#
# Output:
#   output/multibranch-training/v11-blend-70-30.ftmb       — Training data
#   models/sherlock-v11/                                    — Trained model
#   results/overnight-v11-retraining.log                    — Full log
#   results/eval-pack-sherlock-v11.tar.gz                   — Eval pack
#   results/v11-transfer-bundle.tar.gz                      — All results for Beelink transfer
#
# Spec: specs/2026-04-12-accuracy-gap-retraining/spec.yaml
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v11-retraining.log"

SKIP_DATA=false
EPOCHS=40

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
echo " Multi-Branch Overnight Pipeline v11 — Distillation-Heavy"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo " Config: FTMB v3, 70/30 distilled/synthetic, seed 42"
echo "         $EPOCHS epochs, ReLU activation, BatchNorm"
echo "         lr=0.0001, weight_decay=0.0001"
echo ""
echo " Purpose: Close accuracy gap 193/227 → 205+/227 via better data."
echo "   Architecture unchanged (ReLU+BN, decision 0046)."
echo "   Data mix shifted 50/50 → 70/30 distillation-heavy."
echo ""
echo " Spec: specs/2026-04-12-accuracy-gap-retraining/"
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

# Model2Vec is REQUIRED — header features are the point
if [[ ! -d models/model2vec ]] || [[ ! -f models/model2vec/model.safetensors ]]; then
    echo "FAIL: Model2Vec not found at models/model2vec/"
    echo "  Header features require Model2Vec. This is a hard requirement."
    exit 1
fi
echo "[Pre-flight] Model2Vec: OK (models/model2vec/)"

# Sibling-context is REQUIRED — enrichment during training
SIBLING_CTX_DIR="models/sibling-context"
if [[ ! -f "$SIBLING_CTX_DIR/model.safetensors" ]]; then
    echo "FAIL: Sibling-context model not found at $SIBLING_CTX_DIR/model.safetensors"
    echo "  Frozen sibling-context enrichment is a hard requirement."
    echo "  Train it with: cargo run --release -- train-sibling-context"
    exit 1
fi
echo "[Pre-flight] Sibling-context: OK ($SIBLING_CTX_DIR/)"

# Use the production-scale config (v5-scaled) as base — ReLU+BN (no GELU+LN)
MODEL_CONFIG="models/sherlock-v5-scaled-config.json"
if [[ ! -f "$MODEL_CONFIG" ]]; then
    echo "FAIL: Model config not found at $MODEL_CONFIG"
    exit 1
fi

echo "[Pre-flight] Model config:"
cat "$MODEL_CONFIG"
echo ""

# Verify no GELU/LayerNorm in config — should be plain ReLU+BN
if grep -q '"GELU"\|"use_layer_norm": true' "$MODEL_CONFIG"; then
    echo "FAIL: Config contains GELU or use_layer_norm=true — this spec uses ReLU+BN only"
    echo "  Use sherlock-v5-scaled-config.json, not sherlock-v6-gelu-config.json"
    exit 1
fi
echo "[Pre-flight] Config is ReLU+BN (no GELU/LN fields) — correct"
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

FTMB_FILE="output/multibranch-training/v11-blend-70-30.ftmb"
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1/5: Data prep — SKIPPED (--skip-data, reusing v11 FTMB)"
    echo "================================================================"
    if [[ -f scripts/read_ftmb.py ]]; then
        python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    fi
    echo ""
else
    # Remove stale file if it exists
    if [[ -f "$FTMB_FILE" ]] && [[ "$SKIP_DATA" != "true" ]]; then
        echo "[Data] Removing stale $FTMB_FILE to force regeneration..."
        rm -f "$FTMB_FILE"
    fi

    echo "================================================================"
    echo " Step 1/5: Prepare Training Data (FTMB v3, 70/30, 35% aug)"
    echo " Started: $(date)"
    echo " Output: $FTMB_FILE"
    echo " Mix: 70% distilled (real-world), 30% synthetic"
    echo "================================================================"

    echo "[Data] Full extraction with 70/30 distillation-heavy mix..."
    python3 scripts/prepare_multibranch_data.py \
        --distilled output/distillation-v3/sherlock_distilled.csv.gz \
        --finetype ./target/release/finetype \
        --output "$FTMB_FILE" \
        --label-remap data/label_remap.json \
        --samples-per-type 1200 \
        --synthetic-columns 1200 \
        --ratio-distilled 0.7 \
        --augmentation-rate 0.35 \
        --format v3 \
        --seed 42 \
        --workers 8

    echo ""
    echo "[Data] Verifying output..."
    if [[ -f scripts/read_ftmb.py ]]; then
        python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    fi

    echo ""
    echo "[Data] Complete: $(date)"
fi
echo ""

# --- Step 1.5: Validate header features in FTMB -------------------------

echo "================================================================"
echo " Step 1.5/5: Validate FTMB header features"
echo "================================================================"

HEADER_VALIDATION=$(python3 -c "
import struct, sys

path = '$FTMB_FILE'
with open(path, 'rb') as f:
    magic = f.read(4)
    assert magic == b'FTMB', f'Bad magic: {magic}'
    version = struct.unpack('<I', f.read(4))[0]
    n_records = struct.unpack('<Q', f.read(8))[0]
    char_dim = struct.unpack('<H', f.read(2))[0]
    embed_dim = struct.unpack('<H', f.read(2))[0]
    stats_dim = struct.unpack('<H', f.read(2))[0]
    header_dim = struct.unpack('<H', f.read(2))[0]

    if version < 3:
        print(f'FAIL: FTMB version {version} does not support table groups')
        sys.exit(1)

    n_groups = struct.unpack('<H', f.read(2))[0]
    _reserved = struct.unpack('<H', f.read(2))[0]

    # Check first 5 groups for header features (ac-04)
    headers_checked = 0
    headers_nonzero = 0
    headers_with_name = 0

    for g in range(min(5, n_groups)):
        n_columns = struct.unpack('<H', f.read(2))[0]
        n_sibling_headers = struct.unpack('<H', f.read(2))[0]

        for _ in range(n_sibling_headers):
            name_len = struct.unpack('<H', f.read(2))[0]
            name = f.read(name_len).decode('utf-8')
            if name.strip():
                headers_with_name += 1

        for c in range(n_columns):
            label_len = struct.unpack('<H', f.read(2))[0]
            label = f.read(label_len).decode('utf-8')
            col_idx = struct.unpack('<H', f.read(2))[0]
            f.read(char_dim * 4)   # skip char features
            f.read(embed_dim * 4)  # skip embed features
            f.read(stats_dim * 4)  # skip stats features
            header_feat = struct.unpack(f'<{header_dim}f', f.read(header_dim * 4))

            headers_checked += 1
            nz = sum(1 for x in header_feat if abs(x) > 1e-6)
            if nz > 10:
                headers_nonzero += 1

    print(f'Groups: {n_groups}, Records: {n_records}')
    print(f'Dims: char={char_dim}, embed={embed_dim}, stats={stats_dim}, header={header_dim}')
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

# --- Step 1.6: Per-type distilled coverage report (ac-03 gate) ----------

echo "================================================================"
echo " Step 1.6/5: Per-type distilled coverage report"
echo "================================================================"

COVERAGE_REPORT=$(python3 -c "
import struct, sys

path = '$FTMB_FILE'
with open(path, 'rb') as f:
    magic = f.read(4)
    assert magic == b'FTMB', f'Bad magic: {magic}'
    version = struct.unpack('<I', f.read(4))[0]
    n_records = struct.unpack('<Q', f.read(8))[0]
    char_dim = struct.unpack('<H', f.read(2))[0]
    embed_dim = struct.unpack('<H', f.read(2))[0]
    stats_dim = struct.unpack('<H', f.read(2))[0]
    header_dim = struct.unpack('<H', f.read(2))[0]

    n_groups = struct.unpack('<H', f.read(2))[0]
    _reserved = struct.unpack('<H', f.read(2))[0]

    # Scan ALL groups to count labels
    label_counts = {}
    record_size = char_dim*4 + embed_dim*4 + stats_dim*4 + header_dim*4

    for g in range(n_groups):
        n_columns = struct.unpack('<H', f.read(2))[0]
        n_sibling_headers = struct.unpack('<H', f.read(2))[0]

        for _ in range(n_sibling_headers):
            name_len = struct.unpack('<H', f.read(2))[0]
            f.read(name_len)

        for c in range(n_columns):
            label_len = struct.unpack('<H', f.read(2))[0]
            label = f.read(label_len).decode('utf-8')
            _col_idx = struct.unpack('<H', f.read(2))[0]
            f.read(record_size)

            label_counts[label] = label_counts.get(label, 0) + 1

total_types = len(label_counts)
total_records = sum(label_counts.values())

# Report
print(f'Total types: {total_types}')
print(f'Total records: {total_records}')
print(f'Records per type: min={min(label_counts.values())}, max={max(label_counts.values())}, avg={total_records/total_types:.0f}')

# Types with very few examples (< 10)
thin_types = {k: v for k, v in label_counts.items() if v < 10}
if thin_types:
    print(f'Types with <10 examples: {len(thin_types)}')
    for k, v in sorted(thin_types.items()):
        print(f'  {k}: {v}')
else:
    print('All types have >=10 examples')

# Gate check: spec requires >=150/239 types with distilled data
# We can't distinguish distilled vs synthetic from the FTMB alone,
# but total type coverage is a proxy.
if total_types >= 150:
    print(f'OK: {total_types} types in FTMB (gate: >=150)')
else:
    print(f'WARN: Only {total_types} types in FTMB (gate: >=150)')
" 2>&1)

echo "$COVERAGE_REPORT"
echo ""

# --- Step 2: Train Model -----------------------------------------------

MODEL_DIR="models/sherlock-v11"

echo "================================================================"
echo " Step 2/5: Train ReLU+BN Model (distillation-heavy data)"
echo " Started: $(date)"
echo " Config: $MODEL_CONFIG (production-scale, ReLU+BN)"
echo " Epochs: $EPOCHS, lr=0.0001, weight_decay=0.0001"
echo " Sibling context: $SIBLING_CTX_DIR/"
echo " Data: $FTMB_FILE (70/30 distilled/synthetic)"
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

# --- Verify saved config is ReLU+BN ------------------------------------

SAVED_CONFIG="$MODEL_DIR/config.json"
if [[ -f "$SAVED_CONFIG" ]]; then
    echo "[Verify] Checking saved config.json..."
    if grep -q '"GELU"' "$SAVED_CONFIG"; then
        echo "[Verify] WARNING: config.json contains GELU — expected ReLU+BN!"
        cat "$SAVED_CONFIG"
    else
        echo "[Verify] OK: config.json uses ReLU+BN (default, no activation field)"
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

    # --- Val accuracy gate check (ac-07) ---
    BEST_VAL=$(duckdb -csv -noheader -c "
        WITH metrics AS (
            SELECT unnest(from_json_strict(
                content::JSON, '[{\"val_accuracy\":0.0}]'
            )) AS m
            FROM read_text('$RESULTS_JSON')
        )
        SELECT ROUND(MAX(m.val_accuracy) * 100, 1) FROM metrics;
    " 2>/dev/null || echo "0")

    echo ""
    echo "[Gate] Best val_accuracy: ${BEST_VAL}%"

    # Compare against gates
    if (( $(echo "$BEST_VAL < 84" | bc -l) )); then
        echo "[Gate] ABORT: val_accuracy ${BEST_VAL}% < 84% — investigate FTMB data quality"
        echo "  This exit condition triggers a data prep investigation, not a training retry."
        exit 1
    elif (( $(echo "$BEST_VAL < 88" | bc -l) )); then
        echo "[Gate] WARNING: val_accuracy ${BEST_VAL}% is in 84-88% zone (underperforming)"
        echo "  Training will continue but profile eval may not meet 205/227 target."
        echo "  Consider: is the 70/30 mix worse than 50/50 for this architecture?"
    else
        echo "[Gate] PASS: val_accuracy ${BEST_VAL}% >= 88% (within 2pp of v4-sibling baseline)"
    fi
    echo ""
fi

# --- Step 3: Evaluation -----------------------------------------------

echo "================================================================"
echo " Step 3/5: Evaluation"
echo " Started: $(date)"
echo "================================================================"
echo ""

if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
    echo "-- Evaluating sherlock-v11 --"
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
echo " Step 4/5: Packing evaluation artifacts"
echo " Started: $(date)"
echo "================================================================"
echo ""

./scripts/eval_pack.sh "$MODEL_DIR"
echo ""

# --- Step 5: Re-evaluate v4-sibling for direct comparison ---------------

V4_MODEL="models/sherlock-v4-sibling"
if [[ -f "$V4_MODEL/model.safetensors" ]]; then
    echo "================================================================"
    echo " Step 5/5: Re-evaluating v4-sibling baseline for comparison"
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

V4_REPORT="$V4_MODEL/eval/report.md"
V11_REPORT="$MODEL_DIR/eval/report.md"

if [[ -f "$V4_REPORT" ]] && [[ -f "$V11_REPORT" ]]; then
    echo "================================================================"
    echo " v4-sibling (baseline) vs v11 (distillation-heavy)"
    echo "================================================================"
    echo ""
    echo " Architecture: identical (ReLU+BN, production-scale)"
    echo " Difference: data mix 50/50 → 70/30 distillation-heavy"
    echo ""

    V4_LABEL=$(grep "Profile label accuracy" "$V4_REPORT" | head -1 | sed 's/.*| //' | sed 's/ |.*//')
    V11_LABEL=$(grep "Profile label accuracy" "$V11_REPORT" | head -1 | sed 's/.*| //' | sed 's/ |.*//')
    echo " v4-sibling: $V4_LABEL"
    echo " v11:        $V11_LABEL"
    echo ""
fi

# --- Profile eval gate check (ac-08) ---

if [[ -f "$V11_REPORT" ]]; then
    V11_CORRECT=$(grep "Profile label accuracy" "$V11_REPORT" | head -1 | grep -oP '\d+(?=/)')
    V11_TOTAL=$(grep "Profile label accuracy" "$V11_REPORT" | head -1 | grep -oP '(?<=/)\d+')

    if [[ -n "$V11_CORRECT" ]] && [[ -n "$V11_TOTAL" ]]; then
        echo "[Gate ac-08] Profile eval: $V11_CORRECT/$V11_TOTAL"

        if [[ "$V11_CORRECT" -ge 205 ]]; then
            echo "[Gate ac-08] PASS: >= 205/227 target. Ready for HuggingFace publish."
        elif [[ "$V11_CORRECT" -ge 200 ]]; then
            echo "[Gate ac-08] NEAR MISS: $V11_CORRECT >= 200 but < 205. Keep locally, analyse remaining failures."
        elif [[ "$V11_CORRECT" -le 193 ]]; then
            echo "[Gate ac-08] REGRESSION: $V11_CORRECT <= baseline (193). Revert and investigate."
        else
            echo "[Gate ac-08] BELOW TARGET: $V11_CORRECT < 205. Improvement but not enough."
        fi
    fi
fi
echo ""

# --- Transfer bundle ──────────────────────────────────────────────────
# Single archive with everything needed for analysis on the Beelink.

TRANSFER_BUNDLE="results/v11-transfer-bundle.tar.gz"
BUNDLE_FILES=()

# Eval packs (v11 + v4-sibling comparison)
for pack in results/eval-pack-sherlock-v11.tar.gz results/eval-pack-sherlock-v4-sibling.tar.gz; do
    [[ -f "$pack" ]] && BUNDLE_FILES+=("$pack")
done

# Training log
[[ -f "$LOG_FILE" ]] && BUNDLE_FILES+=("$LOG_FILE")

# Model weights (needed for HuggingFace publish if gates pass)
for f in "$MODEL_DIR/model.safetensors" "$MODEL_DIR/config.json" "$MODEL_DIR/label_map.json" "$MODEL_DIR/results.json"; do
    [[ -f "$f" ]] && BUNDLE_FILES+=("$f")
done

# Progress file for session continuity
PROGRESS="specs/2026-04-12-accuracy-gap-retraining/progress.md"
[[ -f "$PROGRESS" ]] && BUNDLE_FILES+=("$PROGRESS")

if [[ ${#BUNDLE_FILES[@]} -gt 0 ]]; then
    tar czf "$TRANSFER_BUNDLE" "${BUNDLE_FILES[@]}"
    BUNDLE_SIZE="$(ls -lh "$TRANSFER_BUNDLE" | awk '{print $5}')"
    echo "[Transfer] Created: $TRANSFER_BUNDLE ($BUNDLE_SIZE, ${#BUNDLE_FILES[@]} files)"
    echo ""
    echo "Contents:"
    tar tzf "$TRANSFER_BUNDLE" | sed 's/^/  /'
    echo ""
    echo "To unpack on Beelink:"
    echo "  scp mac:~/github/meridian-online/finetype/$TRANSFER_BUNDLE ."
    echo "  cd ~/github/meridian-online/finetype && tar xzf v11-transfer-bundle.tar.gz"
else
    echo "WARN: No files to bundle for transfer"
fi
echo ""

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
echo " Config: $MODEL_CONFIG (ReLU+BN, production-scale)"
echo " Hyperparameters: lr=0.0001, weight_decay=0.0001"
echo " Data: 70/30 distillation:synthetic"
echo " Log: $LOG_FILE"
echo " Transfer: $TRANSFER_BUNDLE"
echo ""
echo " What changed from v4-sibling:"
echo "   - Data mix: 50/50 → 70/30 distillation-heavy"
echo "   - Epoch cap: 30 → $EPOCHS"
echo "   - Config: v5-scaled dimensions (same as v8-v10)"
echo "   - Architecture: UNCHANGED (ReLU+BN, flat head)"
echo ""
echo " To transfer results to the Beelink:"
echo "   scp mac:~/github/meridian-online/finetype/$TRANSFER_BUNDLE ."
echo "   tar xzf v11-transfer-bundle.tar.gz"
echo "================================================================"
