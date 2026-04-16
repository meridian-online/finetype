#!/usr/bin/env bash
# scripts/overnight_v13_retraining.sh — v13 retrain with data quality fixes
#
# Retrains multi-branch model (v13) with 4 tiers of improvements from the
# v12 data quality audit retrain brief:
#
#   P1: Distilled data decontamination
#       - state_code→country_code remap fixed (now state_code is own type)
#       - SSN, user_agent distilled rows dropped (all mislabeled)
#       - phone_number, postal_code filtered to keep only valid patterns
#
#   P2: Validation pattern gaps filled
#       - http_method: exact match pattern (100% pass rate for methods)
#       - user_agent: prefix-based pattern (Mozilla/, curl/, etc.)
#       - latitude: range validation [-90, 90]
#       - geohash: minimum length tightened from 4 to 6
#
#   P3: Class balance adjustments
#       - Distilled rows capped at 600/type (was uncapped)
#       - Hard-negative decimal_number columns in [-90, 90] range
#
#   P4: Architecture change
#       - Validation branch: [128, 64] → [192, 128] (merged dim 1006 → 1070)
#       - Per-branch gradient norms logged to results.json
#       - n_classes: 239 → 240 (added state_code type)
#
# What's different from v12:
#   - 240 types (was 239: +state_code)
#   - valid_dim: 240 (was 239)
#   - valid_hidden: [192, 128] (was [128, 64])
#   - merged_dim: 1070 (was 1006)
#   - Data prep: --filter-distilled --distilled-cap 600 --hard-negatives 75
#   - Epochs: 50 (was 40)
#
# Runs on M1 Pro with Metal acceleration. Requires:
#   - Distilled data at output/distillation-v3/sherlock_distilled.csv.gz
#   - Model2Vec at models/model2vec/ (HARD REQUIREMENT)
#   - Sibling-context model at models/sibling-context/ (HARD REQUIREMENT)
#   - Label remap at data/label_remap.json (with state_code fix applied)
#
# Usage:
#   ./scripts/overnight_v13_retraining.sh                          # Full pipeline
#   ./scripts/overnight_v13_retraining.sh --skip-data              # Skip data prep
#   ./scripts/overnight_v13_retraining.sh --skip-data --skip-train # Eval + bundle only
#   ./scripts/overnight_v13_retraining.sh --epochs N               # Override epoch count
#
# Output:
#   output/multibranch-training/v13-blend-70-30.ftmb       — Training data (v4)
#   models/sherlock-v13/                                    — Trained model
#   results/overnight-v13-retraining.log                    — Full log
#   results/eval-pack-sherlock-v13.tar.gz                   — Eval pack
#   results/v13-transfer-bundle.tar.gz                      — All results for Beelink transfer
#
# Spec: specs/2026-04-16-v13-retrain/spec.yaml
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v13-retraining.log"

SKIP_DATA=false
SKIP_TRAIN=false
EPOCHS=50

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
echo " Multi-Branch Overnight Pipeline v13 — Data Quality + Architecture"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo " Config: FTMB v4, 70/30 distilled/synthetic, seed 42"
echo "         $EPOCHS epochs, ReLU activation, BatchNorm"
echo "         lr=0.0001, weight_decay=0.0001"
echo "         5th branch: validation(240) → Dense(192) → Dense(128)"
echo ""
echo " Changes from v12:"
echo "   - P1: Distilled data decontamination (filter + state_code type)"
echo "   - P2: Validation patterns (http_method, user_agent, latitude, geohash)"
echo "   - P3: Distilled cap at 600/type + hard-negative mining"
echo "   - P4: Validation branch [128,64] → [192,128], gradient norms"
echo "   - Types: 239 → 240 (+state_code)"
echo ""
echo " Target: ≥210/227 label accuracy (92.5%+)"
echo " Spec: specs/2026-04-16-v13-retrain/"
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

# v13 config with resized validation branch
MODEL_CONFIG="models/sherlock-v13-config.json"
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
echo "[Pre-flight] Config is ReLU+BN — correct"

# Verify v13 validation branch config
if ! grep -q '"valid_dim": 240' "$MODEL_CONFIG"; then
    echo "FAIL: Config missing valid_dim: 240 (expected 240 for v13 with state_code)"
    exit 1
fi
if ! grep -q '"valid_hidden"' "$MODEL_CONFIG"; then
    echo "FAIL: Config missing valid_hidden"
    exit 1
fi
echo "[Pre-flight] Validation branch config: valid_dim=240, valid_hidden present — correct"

# Verify n_classes = 240
if ! grep -q '"n_classes": 240' "$MODEL_CONFIG"; then
    echo "FAIL: Config has wrong n_classes (expected 240 for v13)"
    exit 1
fi
echo "[Pre-flight] n_classes=240 — correct (239 + state_code)"

# Verify state_code remap is fixed
if grep -q '"geography.region.state_code": "geography.location.country_code"' data/label_remap.json; then
    echo "FAIL: data/label_remap.json still maps state_code→country_code"
    echo "  This should have been fixed before running v13 pipeline."
    exit 1
fi
echo "[Pre-flight] state_code remap: fixed (→ state_code, not country_code)"
echo ""

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
    exit 1
fi
echo "[Pre-flight] Header extraction OK ($HEADER_NONZERO/128 nonzero dimensions)"

# Verify validation feature extraction (v13: 240-dim)
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

if [[ "$VALID_DIM_N" -ne 240 ]]; then
    echo "FAIL: Validation features dimension is $VALID_DIM_N, expected 240"
    exit 1
fi
if [[ "$VALID_KEYS_N" -ne 240 ]]; then
    echo "FAIL: type_index_keys count is $VALID_KEYS_N, expected 240"
    exit 1
fi
if [[ "$VALID_NONZERO" -lt 5 ]]; then
    echo "FAIL: Only $VALID_NONZERO nonzero validation features — validators may not be loading"
    exit 1
fi
echo "[Pre-flight] Validation extraction OK ($VALID_DIM_N dim, $VALID_KEYS_N keys, $VALID_NONZERO nonzero)"
echo ""

# --- Step 1: Prepare Training Data ------------------------------------

FTMB_FILE="output/multibranch-training/v13-blend-70-30.ftmb"
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1/5: Data prep — SKIPPED (--skip-data, reusing v13 FTMB)"
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
    echo " Validation: 240-dim pass-rate features per record"
    echo " NEW: --filter-distilled, --distilled-cap 600, --hard-negatives 75"
    echo "================================================================"

    echo "[Data] Full extraction with data quality fixes..."
    python3 scripts/prepare_multibranch_data.py \
        --distilled output/distillation-v3/sherlock_distilled.csv.gz \
        --finetype ./target/release/finetype \
        --output "$FTMB_FILE" \
        --label-remap data/label_remap.json \
        --samples-per-type 1200 \
        --synthetic-columns 1200 \
        --ratio-distilled 0.7 \
        --augmentation-rate 0.35 \
        --filter-distilled \
        --distilled-cap 600 \
        --hard-negatives 75 \
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

    if valid_dim != 240:
        print(f'FAIL: valid_dim={valid_dim}, expected 240')
        sys.exit(1)

    # Check first 5 groups for header + validation features
    headers_checked = 0
    headers_nonzero = 0
    valid_checked = 0
    valid_nonzero = 0
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

# Report types affected by the distilled cap
CAPPED_THRESHOLD = 600
heavy_types = {k: v for k, v in label_counts.items() if v > CAPPED_THRESHOLD + 100}
if heavy_types:
    print(f'Types still above {CAPPED_THRESHOLD + 100} records (check cap): {len(heavy_types)}')
    for k, v in sorted(heavy_types.items(), key=lambda x: -x[1])[:10]:
        print(f'  {k}: {v}')

if total_types >= 200:
    print(f'OK: {total_types} types in FTMB (gate: >=200)')
else:
    print(f'WARN: Only {total_types} types in FTMB (gate: >=200)')
" 2>&1)

echo "$COVERAGE_REPORT"
echo ""

# --- Step 2: Train Model -----------------------------------------------

MODEL_DIR="models/sherlock-v13"

echo "================================================================"
echo " Step 2/5: Train 5-Branch Model (v13 architecture)"
echo " Started: $(date)"
echo " Config: $MODEL_CONFIG"
echo " Epochs: $EPOCHS, lr=0.0001, weight_decay=0.0001"
echo " Sibling context: $SIBLING_CTX_DIR/"
echo " Data: $FTMB_FILE (FTMB v4, 70/30 distilled/synthetic)"
echo " Branches: char(960) + embed(512) + stats(27) + header(128) + valid(240)"
echo " Architecture: valid_hidden=[192,128], merged_dim=1070"
echo " NEW: Per-branch gradient norms logged to results.json"
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

    # --- Gradient norms summary ---
    echo ""
    echo "Gradient norms (last epoch):"
    python3 -c "
import json
with open('$RESULTS_JSON') as f:
    metrics = json.load(f)
last = metrics[-1]
norms = last.get('branch_gradient_norms', {})
if norms:
    for branch in sorted(norms.keys()):
        print(f'  {branch:8s}: {norms[branch]:.4f}')
else:
    print('  (no gradient norms recorded)')
" 2>/dev/null || echo "  (gradient norms query failed)"

    # --- Val accuracy gate check ---
    BEST_VAL=$(duckdb -csv -noheader -c "
        SELECT ROUND(MAX(val_accuracy) * 100, 1)
        FROM read_json('$RESULTS_JSON', format='array', $RESULTS_COLS);
    " 2>/dev/null || echo "0")

    echo ""
    echo "[Gate] Best val_accuracy: ${BEST_VAL}%"

    # Compare against gates (v13 targets are higher than v12)
    if (( $(echo "$BEST_VAL < 84" | bc -l) )); then
        echo "[Gate] ABORT: val_accuracy ${BEST_VAL}% < 84% — investigate FTMB data quality"
        exit 1
    elif (( $(echo "$BEST_VAL < 88" | bc -l) )); then
        echo "[Gate] WARNING: val_accuracy ${BEST_VAL}% is in 84-88% zone (underperforming)"
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
    echo "-- Evaluating sherlock-v13 --"
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

# --- Step 5: Re-evaluate v12 baseline for comparison --------------------

V12_MODEL="models/sherlock-v12"
if [[ -f "$V12_MODEL/model.safetensors" ]]; then
    echo "================================================================"
    echo " Step 5/5: Re-evaluating v12 baseline for comparison"
    echo " Started: $(date)"
    echo "================================================================"
    echo ""

    ./scripts/eval.sh --model "$V12_MODEL" || {
        echo "WARN: Eval failed for v12 baseline"
    }

    mkdir -p "$V12_MODEL/eval"
    cp -r eval/eval_output/* "$V12_MODEL/eval/" 2>/dev/null || true
    ./scripts/eval_pack.sh "$V12_MODEL"
    echo ""
fi

# --- Comparison -------------------------------------------------------

V12_REPORT="$V12_MODEL/eval/report.md"
V13_REPORT="$MODEL_DIR/eval/report.md"

if [[ -f "$V12_REPORT" ]] && [[ -f "$V13_REPORT" ]]; then
    echo "================================================================"
    echo " v12 (baseline) vs v13 (data quality + architecture)"
    echo "================================================================"
    echo ""
    echo " Architecture: v12 = valid[128,64], v13 = valid[192,128]"
    echo " Data: v13 adds distilled filtering, cap 600/type, hard negatives"
    echo " Types: v12 = 239, v13 = 240 (+state_code)"
    echo ""

    V12_LABEL=$(grep "Profile label accuracy" "$V12_REPORT" | head -1 | sed 's/.*| //' | sed 's/ |.*//')
    V13_LABEL=$(grep "Profile label accuracy" "$V13_REPORT" | head -1 | sed 's/.*| //' | sed 's/ |.*//')
    echo " v12 (baseline):  $V12_LABEL"
    echo " v13 (improved):  $V13_LABEL"
    echo ""
fi

# --- Profile eval gate check ---

if [[ -f "$V13_REPORT" ]]; then
    V13_CORRECT=$(grep "Profile label accuracy" "$V13_REPORT" | head -1 | sed 's/.*| //' | sed 's/\/.*//')
    V13_TOTAL=$(grep "Profile label accuracy" "$V13_REPORT" | head -1 | sed 's/.*\///' | sed 's/ .*//')

    if [[ -n "$V13_CORRECT" ]] && [[ -n "$V13_TOTAL" ]]; then
        echo "[Gate] Profile eval: $V13_CORRECT/$V13_TOTAL"

        if [[ "$V13_CORRECT" -ge 215 ]]; then
            echo "[Gate] EXCEEDS TARGET: >= 215/227. Outstanding result."
        elif [[ "$V13_CORRECT" -ge 210 ]]; then
            echo "[Gate] PASS: >= 210/227 target (92.5%+). Ready for analysis."
        elif [[ "$V13_CORRECT" -le 204 ]]; then
            echo "[Gate] REGRESSION: $V13_CORRECT <= v12 baseline (204). Investigate."
        else
            echo "[Gate] BELOW TARGET: $V13_CORRECT < 210 but above v12. Partial improvement."
        fi
    fi
fi
echo ""

# --- Transfer bundle ──────────────────────────────────────────────────

TRANSFER_BUNDLE="results/v13-transfer-bundle.tar.gz"
BUNDLE_FILES=()

# Eval packs (v13 + v12 comparison)
for pack in results/eval-pack-sherlock-v13.tar.gz results/eval-pack-sherlock-v12.tar.gz; do
    [[ -f "$pack" ]] && BUNDLE_FILES+=("$pack")
done

# Training log
[[ -f "$LOG_FILE" ]] && BUNDLE_FILES+=("$LOG_FILE")

# Model weights
for f in "$MODEL_DIR/model.safetensors" "$MODEL_DIR/config.json" "$MODEL_DIR/label_map.json" "$MODEL_DIR/results.json"; do
    [[ -f "$f" ]] && BUNDLE_FILES+=("$f")
done

# Spec for reference
SPEC="specs/2026-04-16-v13-retrain/spec.yaml"
[[ -f "$SPEC" ]] && BUNDLE_FILES+=("$SPEC")

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
    echo "  cd ~/github/meridian-online/finetype && tar xzf v13-transfer-bundle.tar.gz"
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
echo " Config: $MODEL_CONFIG"
echo " Hyperparameters: lr=0.0001, weight_decay=0.0001, $EPOCHS epochs"
echo " Data: 70/30 distillation:synthetic, FTMB v4"
echo "   Filtered: --filter-distilled (SSN, user_agent dropped)"
echo "   Capped: --distilled-cap 600"
echo "   Hard negatives: --hard-negatives 75 (decimal_number in [-90,90])"
echo " Log: $LOG_FILE"
echo " Transfer: $TRANSFER_BUNDLE"
echo ""
echo " What changed from v12:"
echo "   - Types: 239 → 240 (+state_code)"
echo "   - valid_dim: 239 → 240"
echo "   - valid_hidden: [128,64] → [192,128]"
echo "   - merged_dim: 1006 → 1070"
echo "   - Distilled data: filtered + capped at 600/type"
echo "   - Hard negatives: 75 decimal_number columns in [-90,90]"
echo "   - Gradient norms: per-branch L2 norms in results.json"
echo "   - Epochs: 40 → 50"
echo ""
echo " To transfer results to the Beelink:"
echo "   scp mac:~/github/meridian-online/finetype/$TRANSFER_BUNDLE ."
echo "   tar xzf v13-transfer-bundle.tar.gz"
echo "================================================================"
