#!/usr/bin/env bash
# scripts/overnight_v12_retraining.sh — 5-branch retrain with validation features
#
# Retrain multi-branch model with a 5th validation branch that receives
# 239-dim JSON Schema pass-rate features (one per taxonomy type). The
# validation branch learns patterns like "values that pass country_code
# validation but fail country validation" — signal the 4-branch model
# can't reach via character/embedding/stats/header features alone.
#
# What's different from v11:
#   - FTMB v4 format: 30-byte header, validation features per record
#   - 5th branch: validation(239) → Dense(128) → Dense(64) → merge
#   - Model config: sherlock-v12-config.json (adds valid_dim, valid_hidden)
#   - Post-training: type_index_keys injected into config.json for inference
#
# Architecture changes from v11:
#   - merged_dim: 628 → 692 (+64 from validation branch hidden[1])
#   - New weights: valid_fc1 (239→128), valid_bn1, valid_fc2 (128→64), valid_bn2
#   - merge_fc1 input: 692 (was 628)
#   - All other branches: UNCHANGED (ReLU+BN, flat head)
#
# Runs on M1 Pro with Metal acceleration. Requires:
#   - Distilled data at output/distillation-v3/sherlock_distilled.csv.gz
#   - Model2Vec at models/model2vec/ (HARD REQUIREMENT)
#   - Sibling-context model at models/sibling-context/ (HARD REQUIREMENT)
#   - Label remap at data/label_remap.json
#
# Usage:
#   ./scripts/overnight_v12_retraining.sh                          # Full pipeline
#   ./scripts/overnight_v12_retraining.sh --skip-data              # Skip data prep
#   ./scripts/overnight_v12_retraining.sh --skip-data --skip-train # Eval + bundle only
#   ./scripts/overnight_v12_retraining.sh --epochs N               # Override epoch count
#
# Output:
#   output/multibranch-training/v12-blend-70-30.ftmb       — Training data (v4)
#   models/sherlock-v12/                                    — Trained model
#   results/overnight-v12-retraining.log                    — Full log
#   results/eval-pack-sherlock-v12.tar.gz                   — Eval pack
#   results/v12-transfer-bundle.tar.gz                      — All results for Beelink transfer
#
# Spec: .orbit/specs/2026-04-15-validation-branch-v12/spec.yaml
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v12-retraining.log"

SKIP_DATA=false
SKIP_TRAIN=false
EPOCHS=40

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-data)   SKIP_DATA=true; shift ;;
        --skip-train)  SKIP_TRAIN=true; shift ;;
        --epochs)      EPOCHS="$2"; shift 2 ;;
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
echo " Multi-Branch Overnight Pipeline v12 — Validation Branch"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo " Config: FTMB v4, 70/30 distilled/synthetic, seed 42"
echo "         $EPOCHS epochs, ReLU activation, BatchNorm"
echo "         lr=0.0001, weight_decay=0.0001"
echo "         5th branch: validation(239) → Dense(128) → Dense(64)"
echo ""
echo " Purpose: Close accuracy gap 201/227 → 215+/227 via validation"
echo "   features. The 4-branch model hits ceiling at 201 — remaining"
echo "   26 misclassifications are model-level confusions that"
echo "   character/embedding/stats/header features can't resolve."
echo ""
echo " Spec: .orbit/specs/2026-04-15-validation-branch-v12/"
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

# v12 config with validation branch
MODEL_CONFIG="models/sherlock-v12-config.json"
if [[ ! -f "$MODEL_CONFIG" ]]; then
    echo "FAIL: Model config not found at $MODEL_CONFIG"
    exit 1
fi

echo "[Pre-flight] Model config:"
cat "$MODEL_CONFIG"
echo ""

# Verify no GELU/LayerNorm — ReLU+BN only
if grep -q '"GELU"\|"use_layer_norm": true' "$MODEL_CONFIG"; then
    echo "FAIL: Config contains GELU or use_layer_norm=true — this spec uses ReLU+BN only"
    exit 1
fi
echo "[Pre-flight] Config is ReLU+BN (no GELU/LN fields) — correct"

# Verify validation branch config
if ! grep -q '"valid_dim": 239' "$MODEL_CONFIG"; then
    echo "FAIL: Config missing valid_dim: 239"
    exit 1
fi
if ! grep -q '"valid_hidden"' "$MODEL_CONFIG"; then
    echo "FAIL: Config missing valid_hidden"
    exit 1
fi
echo "[Pre-flight] Validation branch config: valid_dim=239, valid_hidden present — correct"
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

# Verify validation feature extraction (v12-specific)
echo "[Pre-flight] Verifying validation feature extraction..."
VALID_CHECK=$(echo '["US", "GB", "DE", "FR", "JP"]' | \
    ./target/release/finetype extract-features --json --header "country" --validation 2>/dev/null)
VALID_DIM=$(echo "$VALID_CHECK" | python3 -c "
import json, sys
data = json.load(sys.stdin)
vf = data.get('validation', [])
keys = data.get('type_index_keys', [])
nonzero = sum(1 for x in vf if abs(x) > 1e-6)
print(f'{len(vf)} {len(keys)} {nonzero}')
" 2>/dev/null || echo "0 0 0")

VALID_DIM_N=$(echo "$VALID_DIM" | awk '{print $1}')
VALID_KEYS_N=$(echo "$VALID_DIM" | awk '{print $2}')
VALID_NONZERO=$(echo "$VALID_DIM" | awk '{print $3}')

if [[ "$VALID_DIM_N" -ne 239 ]]; then
    echo "FAIL: Validation features dimension is $VALID_DIM_N, expected 239"
    exit 1
fi
if [[ "$VALID_KEYS_N" -ne 239 ]]; then
    echo "FAIL: type_index_keys count is $VALID_KEYS_N, expected 239"
    exit 1
fi
if [[ "$VALID_NONZERO" -lt 5 ]]; then
    echo "FAIL: Only $VALID_NONZERO nonzero validation features — validators may not be loading"
    exit 1
fi
echo "[Pre-flight] Validation extraction OK ($VALID_DIM_N dim, $VALID_KEYS_N keys, $VALID_NONZERO nonzero)"
echo ""

# --- Step 1: Prepare Training Data ------------------------------------

FTMB_FILE="output/multibranch-training/v12-blend-70-30.ftmb"
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1/5: Data prep — SKIPPED (--skip-data, reusing v12 FTMB)"
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
    echo " Step 1/5: Prepare Training Data (FTMB v4, 70/30, 35% aug)"
    echo " Started: $(date)"
    echo " Output: $FTMB_FILE"
    echo " Mix: 70% distilled (real-world), 30% synthetic"
    echo " Validation: 239-dim pass-rate features per record"
    echo "================================================================"

    echo "[Data] Full extraction with 70/30 distillation-heavy mix + validation..."
    python3 scripts/prepare_multibranch_data.py \
        --distilled output/distillation-v3/sherlock_distilled.csv.gz \
        --finetype ./target/release/finetype \
        --output "$FTMB_FILE" \
        --label-remap data/label_remap.json \
        --samples-per-type 1200 \
        --synthetic-columns 1200 \
        --ratio-distilled 0.7 \
        --augmentation-rate 0.35 \
        --format v4 \
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

# --- Step 1.5: Validate FTMB v4 header and features ----------------------

echo "================================================================"
echo " Step 1.5/5: Validate FTMB v4 header and validation features"
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

    if version < 4:
        print(f'FAIL: FTMB version {version}, expected v4')
        sys.exit(1)

    n_groups = struct.unpack('<H', f.read(2))[0]
    _reserved = struct.unpack('<H', f.read(2))[0]
    valid_dim = struct.unpack('<H', f.read(2))[0]

    print(f'FTMB v{version}: {n_records} records, {n_groups} groups')
    print(f'Dims: char={char_dim}, embed={embed_dim}, stats={stats_dim}, header={header_dim}, valid={valid_dim}')

    if valid_dim != 239:
        print(f'FAIL: valid_dim={valid_dim}, expected 239')
        sys.exit(1)

    # Check first 5 groups for header + validation features
    headers_checked = 0
    headers_nonzero = 0
    valid_checked = 0
    valid_nonzero = 0
    headers_with_name = 0

    record_feat_size = char_dim*4 + embed_dim*4 + stats_dim*4 + header_dim*4

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

            # Read char, embed, stats features (skip)
            f.read(char_dim * 4)
            f.read(embed_dim * 4)
            f.read(stats_dim * 4)

            # Read header features
            header_feat = struct.unpack(f'<{header_dim}f', f.read(header_dim * 4))
            headers_checked += 1
            nz = sum(1 for x in header_feat if abs(x) > 1e-6)
            if nz > 10:
                headers_nonzero += 1

            # Read validation features (v4-specific)
            valid_feat = struct.unpack(f'<{valid_dim}f', f.read(valid_dim * 4))
            valid_checked += 1
            vnz = sum(1 for x in valid_feat if abs(x) > 1e-6)
            if vnz > 0:
                valid_nonzero += 1

    print(f'Sibling header names (first 5 groups): {headers_with_name} non-empty')
    print(f'Header features (first 5 groups): {headers_nonzero}/{headers_checked} with >10 nonzero dims')
    print(f'Validation features (first 5 groups): {valid_nonzero}/{valid_checked} with >0 nonzero dims')

    if headers_nonzero == 0:
        print('FAIL: All header features are zeros — Model2Vec not working during data prep')
        sys.exit(1)
    elif headers_nonzero < headers_checked * 0.5:
        print(f'WARN: Only {headers_nonzero}/{headers_checked} headers are non-zero')

    if valid_nonzero == 0:
        print('FAIL: All validation features are zeros — validators not working during data prep')
        sys.exit(1)
    elif valid_nonzero < valid_checked * 0.3:
        print(f'WARN: Only {valid_nonzero}/{valid_checked} records have non-zero validation features')
    else:
        print('OK: Header and validation features are populated')
" 2>&1)

echo "$HEADER_VALIDATION"

if echo "$HEADER_VALIDATION" | grep -q "^FAIL:"; then
    echo ""
    echo "Validation failed. Aborting training."
    exit 1
fi
echo ""

# --- Step 1.6: Per-type distilled coverage report -------------------------

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

    # v4: read valid_dim
    valid_dim = 0
    if version >= 4:
        valid_dim = struct.unpack('<H', f.read(2))[0]

    record_size = char_dim*4 + embed_dim*4 + stats_dim*4 + header_dim*4 + valid_dim*4

    # Scan ALL groups to count labels
    label_counts = {}

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

if total_types >= 150:
    print(f'OK: {total_types} types in FTMB (gate: >=150)')
else:
    print(f'WARN: Only {total_types} types in FTMB (gate: >=150)')
" 2>&1)

echo "$COVERAGE_REPORT"
echo ""

# --- Step 2: Train Model -----------------------------------------------

MODEL_DIR="models/sherlock-v12"

echo "================================================================"
echo " Step 2/5: Train 5-Branch Model (with validation features)"
echo " Started: $(date)"
echo " Config: $MODEL_CONFIG (production-scale, ReLU+BN + validation)"
echo " Epochs: $EPOCHS, lr=0.0001, weight_decay=0.0001"
echo " Sibling context: $SIBLING_CTX_DIR/"
echo " Data: $FTMB_FILE (FTMB v4, 70/30 distilled/synthetic)"
echo " Branches: char(960) + embed(512) + stats(27) + header(128) + valid(239)"
echo "================================================================"

if [[ "$SKIP_TRAIN" == "true" ]] && [[ -f "$MODEL_DIR/model.safetensors" ]]; then
    echo "[Train] --skip-train: using existing model at $MODEL_DIR"
elif [[ -f "$MODEL_DIR/model.safetensors" ]]; then
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

# --- Post-training: inject type_index_keys into config.json ---------------

SAVED_CONFIG="$MODEL_DIR/config.json"
if [[ -f "$SAVED_CONFIG" ]]; then
    echo "[Post-train] Checking saved config.json..."

    # Verify ReLU+BN (no GELU)
    if grep -q '"GELU"' "$SAVED_CONFIG"; then
        echo "[Post-train] WARNING: config.json contains GELU — expected ReLU+BN!"
        cat "$SAVED_CONFIG"
    else
        echo "[Post-train] OK: config.json uses ReLU+BN"
    fi

    # Inject type_index_keys if not already present
    if ! grep -q '"type_index_keys"' "$SAVED_CONFIG"; then
        echo "[Post-train] Injecting type_index_keys into config.json..."

        # Extract keys from the finetype binary (same order used during data prep)
        TYPE_KEYS=$(echo '["test"]' | \
            ./target/release/finetype extract-features --json --header "test" --validation 2>/dev/null | \
            python3 -c "import json, sys; print(json.dumps(json.load(sys.stdin)['type_index_keys']))")

        if [[ -n "$TYPE_KEYS" ]] && [[ "$TYPE_KEYS" != "null" ]]; then
            python3 -c "
import json, sys
with open('$SAVED_CONFIG') as f:
    config = json.load(f)
config['type_index_keys'] = json.loads('$TYPE_KEYS')
with open('$SAVED_CONFIG', 'w') as f:
    json.dump(config, f, indent=2)
    f.write('\n')
print(f'Injected {len(config[\"type_index_keys\"])} type_index_keys')
"
        else
            echo "[Post-train] WARNING: Could not extract type_index_keys — inference will compute them from taxonomy"
        fi
    else
        echo "[Post-train] type_index_keys already present in config.json"
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
    RESULTS_COLS="columns={epoch: 'INTEGER', train_loss: 'FLOAT', val_loss: 'FLOAT', train_accuracy: 'FLOAT', val_accuracy: 'FLOAT', learning_rate: 'DOUBLE', epoch_time_secs: 'FLOAT'}"

    duckdb -c "
        WITH flat AS (
            SELECT * FROM read_json('$RESULTS_JSON', format='array', $RESULTS_COLS)
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

    # --- Val accuracy gate check ---
    BEST_VAL=$(duckdb -csv -noheader -c "
        SELECT ROUND(MAX(val_accuracy) * 100, 1)
        FROM read_json('$RESULTS_JSON', format='array', $RESULTS_COLS);
    " 2>/dev/null || echo "0")

    echo ""
    echo "[Gate] Best val_accuracy: ${BEST_VAL}%"

    # Compare against gates
    if (( $(echo "$BEST_VAL < 84" | bc -l) )); then
        echo "[Gate] ABORT: val_accuracy ${BEST_VAL}% < 84% — investigate FTMB data quality"
        echo "  The 5th branch may be degrading other branches. Check for feature corruption."
        exit 1
    elif (( $(echo "$BEST_VAL < 88" | bc -l) )); then
        echo "[Gate] WARNING: val_accuracy ${BEST_VAL}% is in 84-88% zone (underperforming)"
        echo "  The validation branch may not be helping yet. Check if it's learning (valid_branch loss)."
    else
        echo "[Gate] PASS: val_accuracy ${BEST_VAL}% >= 88%"
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
    echo "-- Evaluating sherlock-v12 --"
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

# --- Step 5: Re-evaluate v11 baseline for comparison --------------------

V11_MODEL="models/sherlock-v11"
if [[ -f "$V11_MODEL/model.safetensors" ]]; then
    echo "================================================================"
    echo " Step 5/5: Re-evaluating v11 baseline for comparison"
    echo " Started: $(date)"
    echo "================================================================"
    echo ""

    ./scripts/eval.sh --model "$V11_MODEL" || {
        echo "WARN: Eval failed for v11 baseline"
    }

    mkdir -p "$V11_MODEL/eval"
    cp -r eval/eval_output/* "$V11_MODEL/eval/" 2>/dev/null || true
    ./scripts/eval_pack.sh "$V11_MODEL"
    echo ""
fi

# --- Comparison -------------------------------------------------------

V11_REPORT="$V11_MODEL/eval/report.md"
V12_REPORT="$MODEL_DIR/eval/report.md"

if [[ -f "$V11_REPORT" ]] && [[ -f "$V12_REPORT" ]]; then
    echo "================================================================"
    echo " v11 (4-branch baseline) vs v12 (5-branch + validation)"
    echo "================================================================"
    echo ""
    echo " Architecture: v11 = 4 branches, v12 = 5 branches (+validation)"
    echo " Data: both 70/30 distillation-heavy, v12 adds 239-dim validation feats"
    echo ""

    V11_LABEL=$(grep "Profile label accuracy" "$V11_REPORT" | head -1 | sed 's/.*| //' | sed 's/ |.*//')
    V12_LABEL=$(grep "Profile label accuracy" "$V12_REPORT" | head -1 | sed 's/.*| //' | sed 's/ |.*//')
    echo " v11 (4-branch): $V11_LABEL"
    echo " v12 (5-branch): $V12_LABEL"
    echo ""
fi

# --- Profile eval gate check (ac-11) ---

if [[ -f "$V12_REPORT" ]]; then
    V12_CORRECT=$(grep "Profile label accuracy" "$V12_REPORT" | head -1 | grep -oP '\d+(?=/)')
    V12_TOTAL=$(grep "Profile label accuracy" "$V12_REPORT" | head -1 | grep -oP '(?<=/)\d+')

    if [[ -n "$V12_CORRECT" ]] && [[ -n "$V12_TOTAL" ]]; then
        echo "[Gate ac-11] Profile eval: $V12_CORRECT/$V12_TOTAL"

        if [[ "$V12_CORRECT" -ge 215 ]]; then
            echo "[Gate ac-11] PASS: >= 215/227 target. Ready for HuggingFace publish."
        elif [[ "$V12_CORRECT" -ge 210 ]]; then
            echo "[Gate ac-11] NEAR MISS: $V12_CORRECT >= 210 but < 215. Keep locally, analyse remaining failures."
        elif [[ "$V12_CORRECT" -le 201 ]]; then
            echo "[Gate ac-11] REGRESSION: $V12_CORRECT <= v11 baseline (201). Validation branch may be hurting."
        else
            echo "[Gate ac-11] BELOW TARGET: $V12_CORRECT < 215. Improvement but not enough."
        fi
    fi
fi
echo ""

# --- Transfer bundle ──────────────────────────────────────────────────
# Single archive with everything needed for analysis on the Beelink.

TRANSFER_BUNDLE="results/v12-transfer-bundle.tar.gz"
BUNDLE_FILES=()

# Eval packs (v12 + v11 comparison)
for pack in results/eval-pack-sherlock-v12.tar.gz results/eval-pack-sherlock-v11.tar.gz; do
    [[ -f "$pack" ]] && BUNDLE_FILES+=("$pack")
done

# Training log
[[ -f "$LOG_FILE" ]] && BUNDLE_FILES+=("$LOG_FILE")

# Model weights (needed for HuggingFace publish if gates pass)
for f in "$MODEL_DIR/model.safetensors" "$MODEL_DIR/config.json" "$MODEL_DIR/label_map.json" "$MODEL_DIR/results.json"; do
    [[ -f "$f" ]] && BUNDLE_FILES+=("$f")
done

# Progress file for session continuity
PROGRESS=".orbit/specs/2026-04-15-validation-branch-v12/progress.md"
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
    echo "  cd ~/github/meridian-online/finetype && tar xzf v12-transfer-bundle.tar.gz"
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
echo " Config: $MODEL_CONFIG (ReLU+BN + validation branch)"
echo " Hyperparameters: lr=0.0001, weight_decay=0.0001"
echo " Data: 70/30 distillation:synthetic, FTMB v4"
echo " Log: $LOG_FILE"
echo " Transfer: $TRANSFER_BUNDLE"
echo ""
echo " What changed from v11:"
echo "   - FTMB: v3 → v4 (adds 239-dim validation features per record)"
echo "   - Branches: 4 → 5 (char + embed + stats + header + validation)"
echo "   - merged_dim: 628 → 692 (+64 from validation branch)"
echo "   - Config: sherlock-v12-config.json"
echo "   - Post-train: type_index_keys injected for inference"
echo "   - Architecture: UNCHANGED for branches 1-4 (ReLU+BN, flat head)"
echo ""
echo " To transfer results to the Beelink:"
echo "   scp mac:~/github/meridian-online/finetype/$TRANSFER_BUNDLE ."
echo "   tar xzf v12-transfer-bundle.tar.gz"
echo "================================================================"
