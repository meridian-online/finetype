#!/usr/bin/env bash
# scripts/overnight_v20_geography.sh — v20 retrain with YDF-specialist geography augmentation
#
# Augments the v19 distilled training corpus with
# eval/gittables/v20_training_candidates/geography.distilled.csv.gz
# (1,537 YDF-confident, holdout-filtered, leakage-firewalled geography
# columns extracted from gittables by
# scripts/extract_ydf_specialist_training_data.py per spec
# 2026-05-23-ydf-specialist-geography). Runs 3 ReLU seeds on the
# augmented data so the comparison against v19's ReLU-relu-s42 baseline
# isolates the data-augmentation effect (architecture, hyperparameters,
# loaders, gates — all held constant).
#
# Closes spec ac-04 (observation): post-train, re-run the m-19 corpus
# pass with the v20-relu-s42 model as the Sense stage, then compare
# eval/gittables/corpus_pass/report.md Part 1 against m-19's baseline
# in the two target cells:
#   - reject_rate_ceil × format_diversity_path_b  (postal_code → full_address)
#   - non_trivial_floor × misclassification       (missed geography labels)
# The ac threshold is ≥20% row-count reduction in BOTH cells.
#
# Usage:
#   ./scripts/overnight_v20_geography.sh                  # Full pipeline
#   ./scripts/overnight_v20_geography.sh --skip-merge     # Reuse existing merged distilled
#   ./scripts/overnight_v20_geography.sh --skip-data      # Skip FTMB prep
#   ./scripts/overnight_v20_geography.sh --dry-run        # Show config, don't train
#   ./scripts/overnight_v20_geography.sh --epochs N       # Override epoch count
#
# Output:
#   output/distillation-v20/sherlock_distilled_with_geography.csv.gz  — merged distilled
#   output/multibranch-training/v20-geography-blend.ftmb               — Training data
#   models/sherlock-v20-geography-relu-s{42,43,44}/                    — Trained models
#   results/overnight-v20-geography.log                                — Full log
#
# Spec: 2026-05-23-ydf-specialist-geography
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v20-geography.log"

SKIP_MERGE=false
SKIP_DATA=false
DRY_RUN=false
EPOCHS=100
PATIENCE=15
BATCH_SIZE=32
SEEDS=(42 43 44)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-merge)  SKIP_MERGE=true; shift ;;
        --skip-data)   SKIP_DATA=true; shift ;;
        --dry-run)     DRY_RUN=true; shift ;;
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

# ── Configuration ──────────────────────────────────────────────────────
RELU_CONFIG="models/sherlock-v13-config.json"
FTMB_FILE="output/multibranch-training/v20-geography-blend.ftmb"
V19_DISTILLED="output/distillation-v3/sherlock_distilled.csv.gz"
GEOGRAPHY_DISTILLED="eval/gittables/v20_training_candidates/geography.distilled.csv.gz"
V20_DISTILLED_DIR="output/distillation-v20"
V20_DISTILLED="$V20_DISTILLED_DIR/sherlock_distilled_with_geography.csv.gz"
HF_DATASET="meridian-online/sherlock-annotated"

# Run definitions: name|config|seed|lr|wd  (3 ReLU seeds — dropping GELU since
# v19 picked ReLU as the production winner)
RUNS=(
    "sherlock-v20-geography-relu-s42|$RELU_CONFIG|42|0.0001|0.0001"
    "sherlock-v20-geography-relu-s43|$RELU_CONFIG|43|0.0001|0.0001"
    "sherlock-v20-geography-relu-s44|$RELU_CONFIG|44|0.0001|0.0001"
)

echo "================================================================"
echo " v20 Geography Retrain — v19 distilled + YDF-specialist geography"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo ""
echo " Augmentation: $GEOGRAPHY_DISTILLED"
echo " Base:         $V19_DISTILLED"
echo " Merged:       $V20_DISTILLED"
echo " Architecture: ReLU+BN ($RELU_CONFIG)"
echo " Sweep:        ${#SEEDS[@]} seeds = ${#RUNS[@]} runs"
echo " Seeds:        ${SEEDS[*]}"
echo " Epochs:       $EPOCHS, patience: $PATIENCE, batch: $BATCH_SIZE"
echo ""
echo " Spec: 2026-05-23-ydf-specialist-geography"
echo " ac-04: ≥20% row-count reduction in both geography cells of"
echo "        eval/gittables/corpus_pass/report.md Part 1 — closed by"
echo "        re-running the m-19 corpus pass with v20-relu-s42 as Sense."
echo "================================================================"
echo ""

if [[ "$DRY_RUN" == "true" ]]; then
    echo "[DRY RUN] Would execute ${#RUNS[@]} training runs:"
    for run_def in "${RUNS[@]}"; do
        IFS='|' read -r name config seed lr wd <<< "$run_def"
        echo "  $name: seed=$seed, lr=$lr, wd=$wd"
        echo "    config: $config"
        echo "    output: models/$name/"
    done
    echo ""
    echo "[DRY RUN] Distilled merge: $V19_DISTILLED + $GEOGRAPHY_DISTILLED -> $V20_DISTILLED"
    echo "[DRY RUN] FTMB: $FTMB_FILE"
    echo "[DRY RUN] Epochs: $EPOCHS, patience: $PATIENCE"
    exit 0
fi

PIPELINE_START=$(date +%s)

# ── Pre-flight checks ─────────────────────────────────────────────────

echo "[Pre-flight] Checking prerequisites..."

if [[ ! -f "$RELU_CONFIG" ]]; then
    echo "FAIL: ReLU config not found: $RELU_CONFIG"
    exit 1
fi

# Verify config has correct dimensions
if ! grep -q '"n_classes": 240' "$RELU_CONFIG"; then
    echo "FAIL: $RELU_CONFIG — n_classes must be 240"
    exit 1
fi
if ! grep -q '"valid_dim": 240' "$RELU_CONFIG"; then
    echo "FAIL: $RELU_CONFIG — valid_dim must be 240"
    exit 1
fi
echo "[Pre-flight] Config: OK"

# Verify geography distilled artefact exists (spec ac-01/ac-02 close evidence).
if [[ ! -f "$GEOGRAPHY_DISTILLED" ]]; then
    echo "FAIL: Geography distilled artefact not found: $GEOGRAPHY_DISTILLED"
    echo "      Generate with: python3 scripts/extract_ydf_specialist_training_data.py --domain geography"
    exit 1
fi
# Sanity-check header and row count. (Reading via Python so the pipeline
# doesn't trip `set -o pipefail` — `gzip -dc | head -1` exits 141 because
# head closes the pipe early, which aborts the script silently.)
GEO_PROBE=$(python3 - <<PY
import gzip, sys
path = "$GEOGRAPHY_DISTILLED"
with gzip.open(path, "rt") as f:
    header = f.readline().rstrip("\n")
    rows = sum(1 for _ in f)
print(header)
print(rows)
PY
)
GEO_HEADER=$(printf '%s\n' "$GEO_PROBE" | sed -n '1p')
GEO_ROWS=$(printf '%s\n' "$GEO_PROBE" | sed -n '2p')
if [[ "$GEO_HEADER" != "final_label,sample_values,column_name" ]]; then
    echo "FAIL: Geography distilled has unexpected header: $GEO_HEADER"
    echo "      Expected: final_label,sample_values,column_name"
    exit 1
fi
echo "[Pre-flight] Geography distilled: $GEO_ROWS rows (schema OK)"

# Confirm leakage firewall ran cleanly on the geography artefact.
GEO_LEAKAGE="eval/gittables/v20_training_candidates/geography.leakage.json"
if [[ -f "$GEO_LEAKAGE" ]]; then
    OVERLAP=$(python3 -c "import json; print(json.load(open('$GEO_LEAKAGE'))['overlap_count'])" 2>/dev/null || echo "?")
    if [[ "$OVERLAP" != "0" ]]; then
        echo "FAIL: Geography leakage report shows overlap_count=$OVERLAP (must be 0)"
        echo "      Re-run the extractor; it should drop any per-value hash collisions."
        exit 1
    fi
    echo "[Pre-flight] Geography leakage: overlap_count=0 (firewall clean)"
else
    echo "WARN: Geography leakage report missing at $GEO_LEAKAGE — proceeding anyway"
fi

# Model2Vec + sibling-context (unchanged from v19)
if [[ ! -d models/model2vec ]] || [[ ! -f models/model2vec/model.safetensors ]]; then
    echo "FAIL: Model2Vec not found at models/model2vec/"
    exit 1
fi
echo "[Pre-flight] Model2Vec: OK"

SIBLING_CTX_DIR="models/sibling-context"
if [[ ! -f "$SIBLING_CTX_DIR/model.safetensors" ]]; then
    echo "FAIL: Sibling-context model not found at $SIBLING_CTX_DIR/"
    exit 1
fi
echo "[Pre-flight] Sibling-context: OK"

# Build with Metal
echo "[Pre-flight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Pre-flight] Build OK"

# Verify feature extraction
echo "[Pre-flight] Verifying header feature extraction..."
HEADER_NONZERO=$(echo '["hello world", "test value", "example"]' | \
    ./target/release/finetype extract-features --json --header "city_name" 2>/dev/null | \
    python3 -c "
import json, sys
data = json.load(sys.stdin)
hf = data.get('header_features', [])
nonzero = sum(1 for x in hf if abs(x) > 1e-6)
print(nonzero)
" 2>/dev/null || echo "0")

if [[ "$HEADER_NONZERO" -lt 10 ]]; then
    echo "FAIL: Header features near-zero ($HEADER_NONZERO/128 nonzero)"
    exit 1
fi
echo "[Pre-flight] Header extraction OK ($HEADER_NONZERO/128 nonzero)"
echo ""

# ── Step 0: Merge v19 distilled + geography distilled (v20 augmentation) ─

mkdir -p "$V20_DISTILLED_DIR"

if [[ "$SKIP_MERGE" == "true" ]] && [[ -f "$V20_DISTILLED" ]]; then
    echo "================================================================"
    echo " Step 0: Distilled merge — SKIPPED (--skip-merge)"
    echo "================================================================"
    MERGED_ROWS=$(gzip -dc < "$V20_DISTILLED" | tail -n +2 | wc -l | tr -d ' ')
    echo "[Merge] Reusing $V20_DISTILLED ($MERGED_ROWS rows)"
else
    # Download v19 base distilled if missing
    if [[ ! -f "$V19_DISTILLED" ]]; then
        echo "[Step 0] v19 distilled missing. Downloading from HuggingFace..."
        mkdir -p output/distillation-v3
        if command -v uv >/dev/null 2>&1; then
            uv run --with datasets python3 -c "
from datasets import load_dataset
import csv, gzip
print('Downloading $HF_DATASET from HuggingFace...')
ds = load_dataset('$HF_DATASET', split='train')
print(f'Downloaded {len(ds)} rows')
with gzip.open('$V19_DISTILLED', 'wt', newline='') as f:
    writer = csv.DictWriter(f, fieldnames=ds.column_names)
    writer.writeheader()
    for row in ds:
        writer.writerow(row)
print(f'Saved {len(ds)} rows to $V19_DISTILLED')
" 2>&1
        else
            echo "FAIL: uv not found for HF download"
            exit 1
        fi
    fi

    echo "================================================================"
    echo " Step 0: Merge v19 distilled + geography distilled"
    echo " Started: $(date)"
    echo "================================================================"

    # The two files have DIFFERENT column sets:
    #   v19 (sherlock):  sherlock_index, split, sample_values, blind_label,
    #                    blind_confidence, finetype_label, finetype_confidence,
    #                    agreement, final_label, reasoning, ground_truth_label
    #   geography:       final_label, sample_values, column_name
    #
    # load_distilled_columns() only reads final_label + sample_values +
    # column_name (column_name absent in sherlock — defaults to ""). So a
    # DictWriter-based merge that emits the v19 superset schema and fills the
    # missing fields on geography rows is safe — the loader ignores the
    # extras and accepts the row.
    python3 - <<PY
import csv, gzip
v19_path = "$V19_DISTILLED"
geo_path = "$GEOGRAPHY_DISTILLED"
out_path = "$V20_DISTILLED"

with gzip.open(v19_path, "rt", newline="") as f:
    v19_reader = csv.DictReader(f)
    v19_fields = v19_reader.fieldnames
# Geography needs final_label + sample_values + column_name; column_name is
# new to the merged file and the loader picks it up where present.
merged_fields = list(v19_fields)
if "column_name" not in merged_fields:
    merged_fields.append("column_name")

n_v19 = 0
n_geo = 0
with gzip.open(out_path, "wt", newline="") as fout:
    writer = csv.DictWriter(fout, fieldnames=merged_fields)
    writer.writeheader()
    with gzip.open(v19_path, "rt", newline="") as f:
        for row in csv.DictReader(f):
            # v19 rows have no column_name — leave it empty (the loader
            # defaults to "" anyway).
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
print(f"merged: v19={n_v19} + geography={n_geo} -> {n_v19 + n_geo} rows at $V20_DISTILLED")
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
    if [[ -f scripts/read_ftmb.py ]]; then
        python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    fi
else
    [[ -f "$FTMB_FILE" ]] && rm -f "$FTMB_FILE"

    echo "================================================================"
    echo " Step 1: Prepare Training Data (FTMB v5, v20 distilled)"
    echo " Started: $(date)"
    echo " Base: v19 recipe (filter_distilled + decontaminate + distilled-cap 600)"
    echo " New: geography rows present in --distilled; subject to per-type cap"
    echo "================================================================"

    python3 scripts/prepare_multibranch_data.py \
        --distilled "$V20_DISTILLED" \
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
    if [[ -f scripts/read_ftmb.py ]]; then
        python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    fi
    echo "[Data] Complete: $(date)"
fi
echo ""

# ── Step 1.5: Pre-training data audit gate (inherited from v19) ───────

echo "================================================================"
echo " Step 1.5: Pre-training Data Audit Gate (v19-inherited)"
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
        print(f'FAIL: FTMB version {version}, expected v4+')
        sys.exit(1)

    n_groups = struct.unpack('<H', f.read(2))[0]
    _reserved = struct.unpack('<H', f.read(2))[0]
    valid_dim = struct.unpack('<H', f.read(2))[0]

    if valid_dim != 240:
        print(f'FAIL: valid_dim={valid_dim}, expected 240')
        sys.exit(1)
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
    print(f'FAIL: Only {total_types} types (need >=238)')
    sys.exit(1)
if total_types < 240:
    try:
        import subprocess as _sp, json as _json
        _tax = _json.loads(_sp.run(['./target/release/finetype', 'taxonomy', '--full', '--output', 'json'],
                                    capture_output=True, text=True).stdout)
        _all = set(e['key'] for e in _tax)
        _missing = sorted(_all - set(label_counts.keys()))
        print(f'  WARN: {len(_missing)} types missing from training: {_missing}')
    except Exception:
        print(f'  WARN: {240 - total_types} types missing (could not enumerate)')
print(f'  PASS')

max_type_count = max(label_counts.values())
print(f'Gate 4: Max type count = {max_type_count} (gate: <=1500)')
if max_type_count > 1500:
    heavy = {k: v for k, v in label_counts.items() if v > 1500}
    print(f'WARN: {len(heavy)} types above 1500')
    for k, v in sorted(heavy.items(), key=lambda kv: -kv[1])[:5]:
        print(f'  {k}: {v}')

_rehabilitated = ['finance.banking.swift_bic', 'technology.internet.http_method',
                  'representation.file.excel_format', 'identity.medical.loinc',
                  'identity.medical.cpt', 'identity.government.ssn',
                  'technology.internet.user_agent']
missing_rehab = [t for t in _rehabilitated if t not in label_counts]
if missing_rehab:
    print(f'WARN: {len(missing_rehab)} v4-rehabilitated types missing: {missing_rehab}')
present_rehab = [t for t in _rehabilitated if t in label_counts]
print(f'Gate 5: {len(present_rehab)}/{len(_rehabilitated)} v4-rehabilitated types present — PASS')

container_types = [k for k in label_counts if k.startswith('container.')]
container_min = min((label_counts[k] for k in container_types), default=0) if container_types else 0
print(f'Gate 6: Container types = {len(container_types)}, min count = {container_min} (gate: >=50)')
if container_min < 50:
    thin_c = {k: label_counts[k] for k in container_types if label_counts[k] < 50}
    if thin_c:
        print(f'FAIL: Container types below 50:')
        for k, v in sorted(thin_c.items()):
            print(f'  {k}: {v}')
        sys.exit(1)
print(f'  PASS')

# Gate 7 (v20-specific): geography types present with non-trivial volume.
# The augmentation adds geography candidate_target_labels; if they get
# filtered out entirely (decontamination, label remap), the v20 retrain
# is no different from v19 and ac-04 has no chance of closing.
geo_types = [k for k in label_counts if k.startswith('geography.')]
geo_total = sum(label_counts[k] for k in geo_types)
print(f'Gate 7 (v20): Geography types = {len(geo_types)}, total rows = {geo_total} (gate: >=200 rows across geography)')
if geo_total < 200:
    print(f'FAIL: geography augmentation appears to have been filtered out;')
    print(f'      v19 baseline had ~{geo_total} geography rows so v20 is no different.')
    sys.exit(1)
print(f'  PASS')

print(f'')
print(f'Audit summary: {total_types} types, {sum(label_counts.values())} records')
print(f'  Per-type range: {min_type_count} — {max_type_count}')
print(f'ALL GATES PASSED')
"
echo ""

# ── Step 2: Training Runs ─────────────────────────────────────────────

RUN_NAMES=()
RUN_STATUSES=()
RUN_TIMES=()

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
    echo " Config: $config"
    echo " Seed: $seed, LR: $lr, WD: $wd"
    echo " Started: $(date)"
    echo "────────────────────────────────────────────────────────────"

    if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
        echo "[Skip] Model already exists at $MODEL_DIR — skipping"
        RUN_STATUSES+=("SKIPPED")
        RUN_TIMES+=("0")
        continue
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
        RUN_STATUSES+=("OK")
        RUN_TIMES+=("$RUN_ELAPSED")

        # Post-training: inject type_index_keys (same as v19)
        SAVED_CONFIG="$MODEL_DIR/config.json"
        if [[ -f "$SAVED_CONFIG" ]] && ! grep -q '"type_index_keys"' "$SAVED_CONFIG"; then
            echo "[Post-train] Injecting type_index_keys..."
            TYPE_KEYS=$(echo '["test"]' | \
                ./target/release/finetype extract-features --json --header "test" --validation 2>/dev/null | \
                python3 -c "import json, sys; print(json.dumps(json.load(sys.stdin)['type_index_keys']))" 2>/dev/null)
            if [[ -n "$TYPE_KEYS" ]] && [[ "$TYPE_KEYS" != "null" ]]; then
                python3 -c "
import json
with open('$SAVED_CONFIG') as f:
    config = json.load(f)
config['type_index_keys'] = json.loads('$TYPE_KEYS')
with open('$SAVED_CONFIG', 'w') as f:
    json.dump(config, f, indent=2)
    f.write('\n')
print(f'Injected {len(config[\"type_index_keys\"])} type_index_keys')
"
            fi
        fi

        echo ""
        echo "[Run $RUN_NUM] COMPLETED in ${RUN_ELAPSED} min"

        RESULTS_JSON="$MODEL_DIR/results.json"
        if [[ -f "$RESULTS_JSON" ]] && command -v duckdb >/dev/null 2>&1; then
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
                    ROUND(best.val_loss, 4) AS best_val_loss
                FROM best;
            " 2>/dev/null || true
        fi
    else
        RUN_END=$(date +%s)
        RUN_ELAPSED=$(( (RUN_END - RUN_START) / 60 ))
        RUN_STATUSES+=("FAILED")
        RUN_TIMES+=("$RUN_ELAPSED")
        echo ""
        echo "[Run $RUN_NUM] FAILED after ${RUN_ELAPSED} min — continuing to next run"
    fi
    echo ""
done

# ── Step 3: Summary ──────────────────────────────────────────────────

PIPELINE_END=$(date +%s)
TOTAL_ELAPSED=$(( (PIPELINE_END - PIPELINE_START) / 60 ))

echo "================================================================"
echo " v20 Geography Retrain — Summary"
echo " Finished: $(date)"
echo " Total elapsed: ${TOTAL_ELAPSED} minutes"
echo "================================================================"
echo ""

printf "%-40s %-10s %-10s\n" "Run" "Status" "Time (min)"
printf "%-40s %-10s %-10s\n" "---" "------" "----------"
for i in "${!RUN_NAMES[@]}"; do
    printf "%-40s %-10s %-10s\n" "${RUN_NAMES[$i]}" "${RUN_STATUSES[$i]:-UNKNOWN}" "${RUN_TIMES[$i]:-?}"
done
echo ""

OK_COUNT=0
for status in "${RUN_STATUSES[@]}"; do
    if [[ "$status" == "OK" || "$status" == "SKIPPED" ]]; then
        OK_COUNT=$((OK_COUNT + 1))
    fi
done

echo "Completed seeds: $OK_COUNT/${#RUNS[@]}"
if [[ $OK_COUNT -lt ${#RUNS[@]} ]]; then
    echo "Some seeds failed; cherry-pick the best completed model before the corpus pass."
fi
echo ""
echo "Next steps to close spec ac-04 (observation):"
echo "  1. Cherry-pick best v20 model (typically sherlock-v20-geography-relu-s42 if all complete)."
echo "  2. Re-run the m-19 corpus pass with v20 as the Sense stage:"
echo "       source eval/gittables/.venv/bin/activate"
echo "       python3 scripts/gittables_corpus_pass.py --jobs 16 --execute  # ~10 hours"
echo "       python3 scripts/gittables_corpus_pass.py --fill-ydf            # ~30 minutes"
echo "  3. Re-build eval/gittables/corpus_pass/report.md."
echo "  4. Compare Part 1 row counts against m-19 baseline in the two cells:"
echo "       - reject_rate_ceil × format_diversity_path_b (postal_code → full_address)"
echo "       - non_trivial_floor × misclassification (missed geography labels)"
echo "  5. If both cells show ≥20% reduction, close ac-04 via 'orbit-acceptance.sh check'."
echo ""
echo "Log: $LOG_FILE"
echo "================================================================"
