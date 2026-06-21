#!/usr/bin/env bash
# scripts/overnight_repro_baseline.sh — spec 2026-06-21 ac-01
#
# The REPRODUCIBLE Model2Vec baseline. 3-seed retrain at v19's EXACT recipe
# (potion-4M embed, 27 column stats, format v4, the v19_paired data blend) but
# at the LIVE taxonomy (244, not v19's frozen 240). Goal: best-of-3 composed
# ~= v19's 0.793 and Sense ~= 0.502 — a MOVABLE baseline that unblocks the
# embed-frontier roadmap. If it misses, the script's per-seed table is the
# drift diagnosis (taxonomy 240->244, data, generators).
#
# Honest metrics (locked, spec ac-01):
#   Sense-vs-Sense   : score_gold_anchor.py predict --raw-model   (no Sharpen)
#   composed (ship)  : score_gold_anchor.py predict               (native Sense+Sharpen)
#
# Build hygiene baked in (both cost builds last session — see ac-00 audit):
#   GATE A  VALID_DIM must track the LIVE taxonomy (derived from `finetype
#           taxonomy`, NOT a hardcoded literal). A stale dim silently rejects
#           every column at inference.
#   GATE B  After the FTMB build, read_ftmb --verify AND assert the record
#           count matches the header (a mid-write crash truncates the binary;
#           --verify reports EOF only as a WARNING, so we fail on it explicitly).
#
# Usage:
#   ./scripts/overnight_repro_baseline.sh                 # full: build data + train 3 seeds + score
#   ./scripts/overnight_repro_baseline.sh --skip-data     # reuse existing FTMB
#   ./scripts/overnight_repro_baseline.sh --score-only    # skip build+train, just score existing models
#   ./scripts/overnight_repro_baseline.sh --dry-run
#   ./scripts/overnight_repro_baseline.sh --epochs N
#
# Output:
#   output/multibranch-training/repro-baseline.ftmb       — training data (FTMB v4, 27 stats, 244 valid)
#   models/repro-baseline-relu-s{42,43,44}/               — the 3 seed models
#   output/embed-frontier/ac01_result.md                  — the deliverable: per-seed Sense + composed
#   results/overnight-repro-baseline.log
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

PY="eval/gittables/.venv/bin/python"           # full scientific stack (numpy/duckdb/pyarrow)
BIN="./target/release/finetype"
LOG_DIR="results"; mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-repro-baseline.log"

CONFIG="models/repro-baseline-config.json"
FTMB_FILE="output/multibranch-training/repro-baseline.ftmb"
DISTILLED_FILE="output/distillation-v3/sherlock_distilled.csv.gz"
HF_DATASET="meridian-online/sherlock-annotated"
GOLD="eval/gold/gold_corpus.tsv"
COLUMNS="eval/gittables/corpus_pass/columns.parquet"
RESULT="output/embed-frontier/ac01_result.md"

SKIP_DATA=false; DRY_RUN=false; SCORE_ONLY=false
EPOCHS=100; PATIENCE=15; BATCH_SIZE=32; LR=0.0001; WD=0.0001; DROPOUT=0.35
SEEDS=(42 43 44)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-data)  SKIP_DATA=true; shift ;;
        --score-only) SCORE_ONLY=true; SKIP_DATA=true; shift ;;
        --dry-run)    DRY_RUN=true; shift ;;
        --epochs)     EPOCHS="$2"; shift 2 ;;
        --help|-h)    sed -n '2,/^set -/p' "$0" | grep '^#' | sed 's/^# \?//'; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

exec > >(tee -a "$LOG_FILE") 2>&1

echo "================================================================"
echo " Reproducible Model2Vec baseline (spec 2026-06-21 ac-01)"
echo " Started: $(date) — $(hostname) $(uname -m)"
echo " Config: $CONFIG (27 stats, 244 valid, potion-4M, ReLU+BN)"
echo " Seeds: ${SEEDS[*]}   Epochs: $EPOCHS   Patience: $PATIENCE"
echo "================================================================"

# ── Build (always — bit-exact 27-stat extractor must match committed source) ──
echo "[Pre-flight] Building finetype (metal release)..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
LIVE_TAXONOMY=$("$BIN" taxonomy 2>/dev/null | grep -cE "^[a-z]")
echo "[Pre-flight] Live taxonomy: $LIVE_TAXONOMY types"
if [[ "$LIVE_TAXONOMY" -lt 200 ]]; then
    echo "FAIL: live taxonomy count looks wrong ($LIVE_TAXONOMY)"; exit 1
fi

# Config must match the live taxonomy on BOTH n_classes and valid_dim.
for field in n_classes valid_dim; do
    val=$("$PY" -c "import json;print(json.load(open('$CONFIG'))['$field'])")
    if [[ "$val" != "$LIVE_TAXONOMY" ]]; then
        echo "FAIL: $CONFIG $field=$val but live taxonomy=$LIVE_TAXONOMY"; exit 1
    fi
done
echo "[Pre-flight] Config n_classes/valid_dim == $LIVE_TAXONOMY — OK"

if [[ "$DRY_RUN" == "true" ]]; then
    echo "[DRY RUN] Would build $FTMB_FILE then train ${#SEEDS[@]} seeds -> models/repro-baseline-relu-s*"
    exit 0
fi

# Pre-flight: shared resources
[[ -f models/model2vec/model.safetensors ]] || { echo "FAIL: models/model2vec missing"; exit 1; }
[[ -f models/sibling-context/model.safetensors ]] || { echo "FAIL: models/sibling-context missing"; exit 1; }
EMBED_VOCAB_DIM=$("$PY" -c "
import json,struct
with open('models/model2vec/model.safetensors','rb') as f:
    n=struct.unpack('<Q',f.read(8))[0]; h=json.loads(f.read(n))
print(h['embeddings']['shape'][1])
")
echo "[Pre-flight] model2vec embed_dim=$EMBED_VOCAB_DIM (expect 128 for potion-4M)"
[[ "$EMBED_VOCAB_DIM" == "128" ]] || { echo "FAIL: expected potion-4M (128-dim) at models/model2vec, got $EMBED_VOCAB_DIM"; exit 1; }

# ── Step 1: Build the FTMB (v19_paired's exact prepare args) ───────────
if [[ "$SCORE_ONLY" == "true" ]]; then
    echo "[Step 1] SCORE-ONLY — skipping data + train"
elif [[ "$SKIP_DATA" == "true" && -f "$FTMB_FILE" ]]; then
    echo "[Step 1] Data prep SKIPPED (--skip-data); reusing $FTMB_FILE"
else
    if [[ ! -f "$DISTILLED_FILE" ]]; then
        echo "[Data] Downloading distilled data from HF ($HF_DATASET)..."
        mkdir -p output/distillation-v3
        uv run --with datasets python3 -c "
from datasets import load_dataset
import csv, gzip
ds = load_dataset('$HF_DATASET', split='train')
with gzip.open('$DISTILLED_FILE','wt',newline='') as f:
    w=csv.DictWriter(f,fieldnames=ds.column_names); w.writeheader()
    for row in ds: w.writerow(row)
print(f'Saved {len(ds)} rows')
"
    fi
    rm -f "$FTMB_FILE"
    echo "[Step 1] Building FTMB (format v4, 27 stats, 244 valid) — $(date)"
    "$PY" scripts/prepare_multibranch_data.py \
        --distilled "$DISTILLED_FILE" \
        --finetype "$BIN" \
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
    echo "[Step 1] FTMB build complete — $(date)"
fi

# ── Step 1.5: Build-hygiene gates A + B ───────────────────────────────
if [[ "$SCORE_ONLY" != "true" ]]; then
    echo "[Gate] Verifying FTMB integrity..."
    [[ -f "$FTMB_FILE" ]] || { echo "FAIL: $FTMB_FILE not found"; exit 1; }

    # read_ftmb --verify: fails (exit 1) on NaN/Inf/dim issues
    VERIFY_OUT=$("$PY" scripts/read_ftmb.py "$FTMB_FILE" --stats --verify 2>&1) || {
        echo "$VERIFY_OUT"; echo "FAIL: read_ftmb --verify reported issues"; exit 1; }
    echo "$VERIFY_OUT" | tail -20

    # GATE B (truncation): --verify reports EOF only as a WARNING, so fail on it.
    if echo "$VERIFY_OUT" | grep -q "WARNING: EOF"; then
        echo "FAIL: FTMB is TRUNCATED (read_ftmb saw EOF before the declared record count)"; exit 1
    fi

    # GATE A (live-taxonomy VALID_DIM): the FTMB header valid_dim must == live taxonomy.
    "$PY" - "$FTMB_FILE" "$LIVE_TAXONOMY" <<'PYGATE'
import struct, sys
path, live = sys.argv[1], int(sys.argv[2])
with open(path, 'rb') as f:
    assert f.read(4) == b'FTMB', 'bad magic'
    version = struct.unpack('<I', f.read(4))[0]
    n_records = struct.unpack('<Q', f.read(8))[0]
    char_dim, embed_dim, stats_dim = struct.unpack('<HHH', f.read(6))
    header_dim = struct.unpack('<H', f.read(2))[0]
    n_groups, _ = struct.unpack('<HH', f.read(4))
    valid_dim = struct.unpack('<H', f.read(2))[0] if version >= 4 else 0
print(f'[Gate] FTMB v{version}: records={n_records} char={char_dim} '
      f'embed={embed_dim} stats={stats_dim} header={header_dim} valid={valid_dim}')
if version < 4:
    print(f'FAIL: FTMB version {version}, expected v4+'); sys.exit(1)
if stats_dim != 27:
    print(f'FAIL: stats_dim={stats_dim}, expected 27 (v19 bit-exact)'); sys.exit(1)
if embed_dim != 512:
    print(f'FAIL: embed_dim={embed_dim}, expected 512 (potion-4M 128x4)'); sys.exit(1)
if valid_dim != live:
    print(f'FAIL: valid_dim={valid_dim} != live taxonomy {live} '
          f'(stale dim silently rejects every column)'); sys.exit(1)
print(f'[Gate A] valid_dim={valid_dim} == live taxonomy {live} — PASS')
PYGATE
    echo "[Gate] FTMB integrity PASS"
fi

# ── Step 2: Train 3 ReLU seeds ────────────────────────────────────────
if [[ "$SCORE_ONLY" != "true" ]]; then
    for seed in "${SEEDS[@]}"; do
        name="repro-baseline-relu-s${seed}"
        MODEL_DIR="models/$name"
        if [[ -f "$MODEL_DIR/model.safetensors" ]]; then
            echo "[Train] $name exists — skipping"; continue
        fi
        echo "──────────────────────────────────────────────"
        echo "[Train] $name  seed=$seed  $(date)"
        echo "──────────────────────────────────────────────"
        cargo run --bin finetype --no-default-features --features metal --release -- \
            train-multi-branch \
            --data "$FTMB_FILE" \
            --output "$MODEL_DIR" \
            --model-config "$CONFIG" \
            --epochs "$EPOCHS" \
            --batch-size "$BATCH_SIZE" \
            --lr "$LR" \
            --weight-decay "$WD" \
            --dropout "$DROPOUT" \
            --seed "$seed" \
            --head flat \
            --patience "$PATIENCE" 2>&1

        # Post-train: inject type_index_keys (label-order contract for inference)
        SAVED_CONFIG="$MODEL_DIR/config.json"
        if [[ -f "$SAVED_CONFIG" ]] && ! grep -q '"type_index_keys"' "$SAVED_CONFIG"; then
            TYPE_KEYS=$(echo '["test"]' | "$BIN" extract-features --json --header "test" --validation 2>/dev/null | \
                "$PY" -c "import json,sys; print(json.dumps(json.load(sys.stdin)['type_index_keys']))" 2>/dev/null)
            if [[ -n "$TYPE_KEYS" && "$TYPE_KEYS" != "null" ]]; then
                "$PY" -c "
import json
c=json.load(open('$SAVED_CONFIG')); c['type_index_keys']=json.loads('$TYPE_KEYS')
json.dump(c,open('$SAVED_CONFIG','w'),indent=2); open('$SAVED_CONFIG','a').write('\n')
print(f'Injected {len(c[\"type_index_keys\"])} type_index_keys')
"
            fi
        fi
        echo "[Train] $name done — $(date)"
    done
fi

# ── Step 3: Score (Sense-vs-Sense + composed) ─────────────────────────
echo "================================================================"
echo " Step 3: Scoring — $(date)"
echo "================================================================"
mkdir -p output/embed-frontier/preds /tmp/repro_score

# headline_of <predictions.tsv> <reframe-flag>
score_headline() {
    local preds="$1" reframe="$2"
    rm -f /tmp/repro_score/report_*.md 2>/dev/null
    "$PY" scripts/score_gold_anchor.py score --gold "$GOLD" --predictions "$preds" \
        --model-name t --out-dir /tmp/repro_score $reframe >/dev/null 2>&1 || true
    grep -hE "Headline" /tmp/repro_score/report_*.md 2>/dev/null | head -1 | grep -oE "[0-9]+\.[0-9]+" | head -1
}

# predict_and_score <model-dir> <name>  → echoes "name | sense | composed"
eval_model() {
    local mdir="$1" nm="$2"
    local p_sense="output/embed-frontier/preds/${nm}_sense.tsv"
    local p_comp="output/embed-frontier/preds/${nm}_composed.tsv"
    FINETYPE_MODEL="$mdir" "$PY" scripts/score_gold_anchor.py predict --raw-model \
        --gold "$GOLD" --columns "$COLUMNS" --binary "$BIN" --out "$p_sense" 2>/dev/null || true
    FINETYPE_MODEL="$mdir" "$PY" scripts/score_gold_anchor.py predict \
        --gold "$GOLD" --columns "$COLUMNS" --binary "$BIN" --out "$p_comp" 2>/dev/null || true
    local s c; s=$(score_headline "$p_sense" ""); c=$(score_headline "$p_comp" "")
    echo "${nm} | ${s:-ERR} | ${c:-ERR}"
}

{
echo "# ac-01 — Reproducible Model2Vec baseline ($(date))"
echo
echo "Recipe: potion-4M, 27 stats, format v4, 244 taxonomy, ReLU+BN, v19_paired data blend."
echo "Honest gate: Sense = predict --raw-model; composed = native profile. Reference: v19 Sense 0.502 / composed 0.793."
echo
echo "| model | Sense (raw) | composed |"
echo "|---|---|---|"
eval_model "models/sherlock-v19-relu-s42" "v19-relu-s42 (reference)"
for seed in "${SEEDS[@]}"; do
    md="models/repro-baseline-relu-s${seed}"
    [[ -f "$md/model.safetensors" ]] && eval_model "$md" "repro-s${seed}" || echo "repro-s${seed} | MISSING | MISSING"
done
echo
echo "Verdict: best-of-3 composed within v19's CI (~0.793) => reproducible movable baseline."
echo "If short: the per-seed table is the drift diagnosis (taxonomy 240->244 is the prime suspect — v19 was 240-dim)."
echo "Done $(date)"
} | tee "$RESULT"

echo "[Done] Result: $RESULT — $(date)"
