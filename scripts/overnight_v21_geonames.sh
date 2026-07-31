#!/usr/bin/env bash
# scripts/overnight_v21_geonames.sh — v21 retrain with GeoNames-sourced geography
#
# Per spec 2026-05-24-v21-geonames-geography ac-05.
#
# Replaces v20's YDF-specialist-extracted geography augmentation (which
# missed cell-2 — see spec close note for the diagnostic) with
# GeoNames-derived training rows. The generator
# (scripts/generate_geonames_geography.py) covers
# geography.{location.*, address.postal_code, coordinate.*} across 15
# launch locales (en/fr/es/de/it/pt/ja/zh/ko/ar/ru/nl/pl + en_GB +
# es_419) with a noise-recipe layer for real-world messiness.
#
# Runs 3 ReLU seeds (42/43/44) at the v13 ReLU+BN config — architecture
# and hyperparameters identical to v19/v20 so the comparison isolates
# the data-source effect.
#
# Closes spec ac-06 (corpus pass) and ac-07 (≥20% cell-2 reduction)
# downstream — this script ships the trained models; the corpus pass is
# the next-day step.
#
# Usage:
#   ./scripts/overnight_v21_geonames.sh                  # Full pipeline
#   ./scripts/overnight_v21_geonames.sh --skip-merge     # Reuse merged distilled
#   ./scripts/overnight_v21_geonames.sh --skip-data      # Skip FTMB prep
#   ./scripts/overnight_v21_geonames.sh --skip-generator # Skip GeoNames generation
#   ./scripts/overnight_v21_geonames.sh --dry-run        # Show config, don't train
#   ./scripts/overnight_v21_geonames.sh --epochs N       # Override epoch count
#
# Output:
#   output/distillation-v21-geonames/geonames_geography.csv.gz  — generator output
#   output/distillation-v21/sherlock_distilled_with_geonames.csv.gz — merged
#   output/multibranch-training/v21-geonames-blend.ftmb          — training data
#   models/sherlock-v21-geonames-geography-relu-s{42,43,44}/     — trained models
#   results/overnight-v21-geonames.log                           — full log
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v21-geonames.log"

SKIP_MERGE=false
SKIP_DATA=false
SKIP_GENERATOR=false
DRY_RUN=false
EPOCHS=100
PATIENCE=15
BATCH_SIZE=32
SEEDS=(42 43 44)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-merge)     SKIP_MERGE=true; shift ;;
        --skip-data)      SKIP_DATA=true; shift ;;
        --skip-generator) SKIP_GENERATOR=true; shift ;;
        --dry-run)        DRY_RUN=true; shift ;;
        --epochs)         EPOCHS="$2"; shift 2 ;;
        --help|-h)
            sed -n '2,/^set -/p' "$0" | grep '^#' | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# Tee all output to log
exec > >(tee -a "$LOG_FILE") 2>&1

# ── Configuration ──────────────────────────────────────────────────────
RELU_CONFIG="models/sherlock-v13-config.json"
FTMB_FILE="output/multibranch-training/v21-geonames-blend.ftmb"
V19_DISTILLED="output/distillation-v3/sherlock_distilled.csv.gz"
GEONAMES_GENERATED="output/distillation-v21-geonames/geonames_geography.csv.gz"
V21_DISTILLED_DIR="output/distillation-v21"
V21_DISTILLED="$V21_DISTILLED_DIR/sherlock_distilled_with_geonames.csv.gz"
HF_DATASET="meridian-online/sherlock-annotated"

RUNS=(
    "sherlock-v21-geonames-geography-relu-s42|$RELU_CONFIG|42|0.0001|0.0001"
    "sherlock-v21-geonames-geography-relu-s43|$RELU_CONFIG|43|0.0001|0.0001"
    "sherlock-v21-geonames-geography-relu-s44|$RELU_CONFIG|44|0.0001|0.0001"
)

echo "================================================================"
echo " v21 GeoNames Retrain — v19 distilled + GeoNames-sourced geography"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo ""
echo " Generator:    $GEONAMES_GENERATED"
echo " Base:         $V19_DISTILLED"
echo " Merged:       $V21_DISTILLED"
echo " Architecture: ReLU+BN ($RELU_CONFIG)"
echo " Seeds:        ${SEEDS[*]}"
echo " Epochs:       $EPOCHS, patience: $PATIENCE, batch: $BATCH_SIZE"
echo ""
echo " Spec: 2026-05-24-v21-geonames-geography"
echo " ac-07: cell-2 lift ≥20% — closed downstream by the corpus pass."
echo "================================================================"
echo ""

if [[ "$DRY_RUN" == "true" ]]; then
    echo "[DRY RUN] Would execute ${#RUNS[@]} training runs:"
    for run_def in "${RUNS[@]}"; do
        IFS='|' read -r name config seed lr wd <<< "$run_def"
        echo "  $name: seed=$seed, lr=$lr, wd=$wd"
    done
    exit 0
fi

PIPELINE_START=$(date +%s)

# ── Pre-flight checks ─────────────────────────────────────────────────
echo "[Pre-flight] Checking prerequisites..."

if [[ ! -f "$RELU_CONFIG" ]]; then
    echo "FAIL: ReLU config not found: $RELU_CONFIG"
    exit 1
fi
grep -q '"n_classes": 240' "$RELU_CONFIG" || { echo "FAIL: $RELU_CONFIG — n_classes must be 240"; exit 1; }
grep -q '"valid_dim": 240' "$RELU_CONFIG" || { echo "FAIL: $RELU_CONFIG — valid_dim must be 240"; exit 1; }
echo "[Pre-flight] Config: OK"

# Dataset integrity — verify all registered training sources before
# committing to the multi-hour pipeline. Per choice 0090.
echo "[Pre-flight] Verifying registered training datasets..."
if ! python3 scripts/dataset_verify.py sherlock cldr geonames 2>&1 | tail -10; then
    echo "FAIL: dataset_verify found drift — abort before training"
    exit 1
fi
echo "[Pre-flight] Datasets: OK"

# Sibling-context model
SIBLING_CTX_DIR="models/sibling-context"
if [[ ! -f "$SIBLING_CTX_DIR/model.safetensors" ]]; then
    echo "FAIL: Sibling-context model not found at $SIBLING_CTX_DIR/"
    exit 1
fi
echo "[Pre-flight] Sibling-context: OK"

# Build binary
echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"

# Verify header feature extraction (catches regressions in the header branch)
HEADER_NONZERO=$(echo '["hello world", "test value", "example"]' | \
    ./target/release/finetype extract-features --json --header "city_name" 2>/dev/null | \
    python3 -c "
import json, sys
data = json.load(sys.stdin)
hf = data.get('header_features', [])
print(sum(1 for x in hf if abs(x) > 1e-6))
" 2>/dev/null || echo "0")
if [[ "$HEADER_NONZERO" -lt 10 ]]; then
    echo "FAIL: Header features near-zero ($HEADER_NONZERO/128 nonzero)"
    exit 1
fi
echo "[Pre-flight] Header extraction OK ($HEADER_NONZERO/128 nonzero)"
echo ""

# ── Step -1: Run the GeoNames generator (if needed) ───────────────────
if [[ "$SKIP_GENERATOR" == "true" ]] && [[ -f "$GEONAMES_GENERATED" ]]; then
    echo "================================================================"
    echo " Step -1: GeoNames generation — SKIPPED (--skip-generator)"
    echo "================================================================"
else
    echo "================================================================"
    echo " Step -1: Generate GeoNames geography training rows"
    echo " Started: $(date)"
    echo "================================================================"
    python3 scripts/generate_geonames_geography.py 2>&1 | tail -8
    python3 scripts/test_geonames_generator.py || { echo "FAIL: generator sanity test"; exit 1; }
    echo "[Step -1] Complete: $(date)"
fi
echo ""

# ── Step 0: Merge v19 distilled + GeoNames distilled (v21 augmentation) ─
mkdir -p "$V21_DISTILLED_DIR"

if [[ "$SKIP_MERGE" == "true" ]] && [[ -f "$V21_DISTILLED" ]]; then
    echo "================================================================"
    echo " Step 0: Distilled merge — SKIPPED (--skip-merge)"
    echo "================================================================"
    MERGED_ROWS=$(python3 -c "import gzip; print(sum(1 for _ in gzip.open('$V21_DISTILLED', 'rt')) - 1)")
    echo "[Merge] Reusing $V21_DISTILLED ($MERGED_ROWS rows)"
else
    # Download v19 base distilled if missing
    if [[ ! -f "$V19_DISTILLED" ]]; then
        echo "[Step 0] v19 distilled missing. Downloading from HuggingFace..."
        mkdir -p output/distillation-v3
        if command -v uv >/dev/null 2>&1; then
            uv run --with datasets python3 -c "
from datasets import load_dataset
import csv, gzip
ds = load_dataset('$HF_DATASET', split='train')
with gzip.open('$V19_DISTILLED', 'wt', newline='') as f:
    writer = csv.DictWriter(f, fieldnames=ds.column_names)
    writer.writeheader()
    for row in ds: writer.writerow(row)
print(f'Saved {len(ds)} rows to $V19_DISTILLED')
"
        else
            echo "FAIL: uv not found for HF download"
            exit 1
        fi
    fi

    echo "================================================================"
    echo " Step 0: Merge v19 distilled + GeoNames generator output"
    echo " Started: $(date)"
    echo "================================================================"

    python3 - <<PY
import csv, gzip
v19_path = "$V19_DISTILLED"
geo_path = "$GEONAMES_GENERATED"
out_path = "$V21_DISTILLED"

with gzip.open(v19_path, "rt", newline="") as f:
    v19_reader = csv.DictReader(f)
    v19_fields = list(v19_reader.fieldnames)
merged_fields = list(v19_fields)
if "column_name" not in merged_fields:
    merged_fields.append("column_name")

n_v19 = n_geo = 0
with gzip.open(out_path, "wt", newline="") as fout:
    writer = csv.DictWriter(fout, fieldnames=merged_fields)
    writer.writeheader()
    with gzip.open(v19_path, "rt", newline="") as f:
        for row in csv.DictReader(f):
            row.setdefault("column_name", "")
            writer.writerow(row)
            n_v19 += 1
    with gzip.open(geo_path, "rt", newline="") as f:
        for row in csv.DictReader(f):
            out_row = {k: "" for k in merged_fields}
            out_row["final_label"] = row["final_label"]
            out_row["sample_values"] = row["sample_values"]
            out_row["column_name"] = row["column_name"]
            writer.writerow(out_row)
            n_geo += 1
print(f"merged: v19={n_v19} + geonames={n_geo} -> {n_v19 + n_geo} rows at $V21_DISTILLED")
PY
    echo "[Step 0] Complete: $(date)"
fi
echo ""

# ── Step 1: Prepare Training Data ─────────────────────────────────────
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1: Data prep — SKIPPED (--skip-data)"
    echo "================================================================"
    [[ -f scripts/read_ftmb.py ]] && python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
else
    [[ -f "$FTMB_FILE" ]] && rm -f "$FTMB_FILE"

    echo "================================================================"
    echo " Step 1: Prepare Training Data (FTMB v5, v21 distilled)"
    echo " Started: $(date)"
    echo " Base: v19 recipe (filter_distilled + decontaminate)"
    echo " Cap:  --distilled-cap 600 (geography raised to 3000 via DOMAIN_CAP_OVERRIDES)"
    echo "================================================================"

    python3 scripts/prepare_multibranch_data.py \
        --distilled "$V21_DISTILLED" \
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
        --seed 42 \
        --workers 8

    echo ""
    echo "[Data] Verifying output..."
    [[ -f scripts/read_ftmb.py ]] && python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    echo "[Data] Complete: $(date)"
fi
echo ""

# ── Step 1.5: Pre-training data audit gate (inherited from v19/v20) ───
echo "================================================================"
echo " Step 1.5: Pre-training Data Audit Gate (v19/v20-inherited + v21 geography lift)"
echo "================================================================"

python3 -c "
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
        print(f'FAIL: FTMB version {version}, expected v4+'); sys.exit(1)
    n_groups = struct.unpack('<H', f.read(2))[0]
    _reserved = struct.unpack('<H', f.read(2))[0]
    valid_dim = struct.unpack('<H', f.read(2))[0]
    if valid_dim != 240:
        print(f'FAIL: valid_dim={valid_dim}, expected 240'); sys.exit(1)
    print(f'Gate 1: valid_dim=240 — PASS')
    record_size = char_dim*4 + embed_dim*4 + stats_dim*4 + header_dim*4 + valid_dim*4

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

min_type_count = min(label_counts.values())
print(f'Gate 2: Min type count = {min_type_count} (gate: >=50)')
if min_type_count < 50:
    thin = {k: v for k, v in label_counts.items() if v < 50}
    print(f'FAIL: {len(thin)} types below 50:')
    for k, v in sorted(thin.items()):
        print(f'  {k}: {v}')
    sys.exit(1)
print(f'  PASS')

total_types = len(label_counts)
print(f'Gate 3: Total types = {total_types} (gate: >=238)')
if total_types < 238:
    print(f'FAIL: only {total_types} types (need >=238)')
    sys.exit(1)
print(f'  PASS')

# Gate 7 (v21-specific): geography types present with v20+ volume.
# v20's gate was 'geography total >= 200'. v21's GeoNames generator emits
# 30k columns; even after dedup+cap we expect >= 3000.
geo_types = [k for k in label_counts if k.startswith('geography.')]
geo_total = sum(label_counts[k] for k in geo_types)
print(f'Gate 7 (v21): Geography types = {len(geo_types)}, total rows = {geo_total} (gate: >=2000 — GeoNames lift)')
if geo_total < 2000:
    print(f'FAIL: geography volume after blending is {geo_total} — GeoNames augmentation lost in pipeline')
    print(f'      Check DOMAIN_CAP_OVERRIDES + decontamination + label_remap.json drift')
    sys.exit(1)
print(f'  PASS — geography lift confirmed: {geo_total} rows across {len(geo_types)} types')
print(f'')
print(f'Audit summary: {total_types} types, {sum(label_counts.values())} records')
print(f'ALL GATES PASSED')
"
echo ""

# ── Step 2: Training Runs ─────────────────────────────────────────────
RUN_NAMES=(); RUN_STATUSES=(); RUN_TIMES=()

echo "================================================================"
echo " Step 2: Training — ${#RUNS[@]} runs"
echo " Started: $(date)"
echo "================================================================"
echo ""

RUN_NUM=0
for run_def in "${RUNS[@]}"; do
    IFS='|' read -r name config seed lr wd <<< "$run_def"
    RUN_NAMES+=("$name")
    RUN_NUM=$((RUN_NUM + 1))
    MODEL_DIR="models/$name"

    echo "────────────────────────────────────────────────────────────"
    echo " Run $RUN_NUM/${#RUNS[@]}: $name (ReLU+BN)"
    echo " Config: $config | Seed: $seed | LR: $lr | WD: $wd"
    echo " Started: $(date)"
    echo "────────────────────────────────────────────────────────────"

    if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
        echo "[Skip] Model already exists at $MODEL_DIR — skipping"
        RUN_STATUSES+=("SKIPPED"); RUN_TIMES+=("0"); continue
    fi

    RUN_START=$(date +%s)

    if cargo run --bin finetype --no-default-features --features metal --release -- \
        train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$MODEL_DIR" \
        --model-config "$config" \
        --epochs "$EPOCHS" \
        --batch-size "$BATCH_SIZE" \
        --lr "$lr" \
        --weight-decay "$wd" \
        --dropout 0.35 \
        --seed "$seed" \
        --head flat \
        --patience "$PATIENCE" \
        2>&1; then

        RUN_END=$(date +%s)
        RUN_ELAPSED=$(( (RUN_END - RUN_START) / 60 ))
        RUN_STATUSES+=("OK"); RUN_TIMES+=("$RUN_ELAPSED")

        # Inject type_index_keys post-train (same as v19/v20)
        SAVED_CONFIG="$MODEL_DIR/config.json"
        if [[ -f "$SAVED_CONFIG" ]] && ! grep -q '"type_index_keys"' "$SAVED_CONFIG"; then
            TYPE_KEYS=$(echo '["test"]' | \
                ./target/release/finetype extract-features --json --header "test" --validation 2>/dev/null | \
                python3 -c "import json, sys; print(json.dumps(json.load(sys.stdin)['type_index_keys']))" 2>/dev/null)
            if [[ -n "$TYPE_KEYS" ]] && [[ "$TYPE_KEYS" != "null" ]]; then
                python3 -c "
import json
with open('$SAVED_CONFIG') as f: config = json.load(f)
config['type_index_keys'] = json.loads('$TYPE_KEYS')
with open('$SAVED_CONFIG', 'w') as f: json.dump(config, f, indent=2); f.write('\n')
"
            fi
        fi
        echo "[Run $RUN_NUM] COMPLETED in ${RUN_ELAPSED} min"
    else
        RUN_END=$(date +%s)
        RUN_ELAPSED=$(( (RUN_END - RUN_START) / 60 ))
        RUN_STATUSES+=("FAILED"); RUN_TIMES+=("$RUN_ELAPSED")
        echo "[Run $RUN_NUM] FAILED after ${RUN_ELAPSED} min — continuing"
    fi
    echo ""
done

# ── Step 3: Summary ──────────────────────────────────────────────────
PIPELINE_END=$(date +%s)
TOTAL_ELAPSED=$(( (PIPELINE_END - PIPELINE_START) / 60 ))

echo "================================================================"
echo " v21 GeoNames Retrain — Summary"
echo " Finished: $(date)"
echo " Total elapsed: ${TOTAL_ELAPSED} minutes"
echo "================================================================"
printf "%-50s %-10s %-10s\n" "Run" "Status" "Time (min)"
printf "%-50s %-10s %-10s\n" "---" "------" "----------"
for i in "${!RUN_NAMES[@]}"; do
    printf "%-50s %-10s %-10s\n" "${RUN_NAMES[$i]}" "${RUN_STATUSES[$i]:-UNKNOWN}" "${RUN_TIMES[$i]:-?}"
done
echo ""

OK_COUNT=0
for status in "${RUN_STATUSES[@]}"; do
    [[ "$status" == "OK" || "$status" == "SKIPPED" ]] && OK_COUNT=$((OK_COUNT + 1))
done

echo "Completed seeds: $OK_COUNT/${#RUNS[@]}"
echo ""
echo "Next steps (spec ac-06, ac-07):"
echo "  1. Cherry-pick best v21 model (typically seed-42):"
echo "       ln -sfn sherlock-v21-geonames-geography-relu-s42 models/default"
echo "  2. Run full corpus pass to a fresh output dir:"
echo "       source eval/gittables/.venv/bin/activate"
echo "       python3 scripts/gittables_corpus_pass.py --jobs 16 --execute --out-dir output/corpus-pass-v21  # ~7h"
echo "       python3 scripts/gittables_corpus_pass.py --fill-ydf --out-dir output/corpus-pass-v21          # ~25min"
echo "  3. Compute three-way cell deltas (v19 / v20 / v21) — see ac-06."
echo "  4. If cell 2 v21/v19 ≤ 0.80 → ac-07 closes; partial close at 10–20% lift."
echo ""
echo "Log: $LOG_FILE"
echo "================================================================"
