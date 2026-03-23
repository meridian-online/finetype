#!/usr/bin/env bash
# scripts/overnight_sherlock.sh — Overnight multi-branch training pipeline (v3)
#
# Runs on M1 Pro with Metal acceleration. Expects:
#   - Distilled data at output/distillation-v3/sherlock_distilled.csv.gz
#   - Model2Vec at models/model2vec/
#   - Label remap at data/label_remap.json
#
# Pipeline:
#   1. Prepare 300k+ column-level training data (.ftmb) — 50/50 distilled/synthetic
#   2. Train flat multi-branch model (20 epochs)
#   3. Train hierarchical multi-branch model (20 epochs)
#   4. Train no-header ablation model (20 epochs, embed features zeroed)
#   5. Evaluate all models against Tier 1 profile eval + ablation delta
#
# Usage:
#   ./scripts/overnight_sherlock.sh                  # Full pipeline
#   ./scripts/overnight_sherlock.sh --flat-only       # Skip hierarchical + ablation
#   ./scripts/overnight_sherlock.sh --skip-data        # Skip data prep (reuse existing .ftmb)
#   ./scripts/overnight_sherlock.sh --skip-ablation    # Skip no-header ablation variant
#
# Output:
#   output/multibranch-training/blend-50-50.ftmb      — Training data (50/50 mix)
#   output/multibranch-training/blend-50-50-noheader.ftmb — Ablation data (headers zeroed)
#   models/sherlock-v3-flat/                           — Flat model + eval
#   models/sherlock-v3-hier/                           — Hierarchical model + eval
#   models/sherlock-v3-noheader/                       — No-header ablation model + eval
#   results/overnight-v3.log                           — Full log
#
# v2 config (preserved for reproducibility):
#   --ratio-distilled 0.3, blend-30-70.ftmb, sherlock-v2-{flat,hier}/
#   To reproduce v2: git checkout <v2-commit> -- scripts/overnight_sherlock.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

LOG_DIR="results"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/overnight-v3.log"

FLAT_ONLY=false
SKIP_DATA=false
SKIP_ABLATION=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --flat-only)      FLAT_ONLY=true; shift ;;
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
echo " Multi-Branch Overnight Pipeline v3"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo " Config: 50/50 distilled/synthetic, seed 42, batch 32, 20 epochs"
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

# --- Step 1: Prepare Training Data (50/50 mix) -----------------------

FTMB_FILE="output/multibranch-training/blend-50-50.ftmb"
FTMB_NOHEADER="output/multibranch-training/blend-50-50-noheader.ftmb"
mkdir -p output/multibranch-training

if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Step 1/5: Data prep — SKIPPED (--skip-data, reusing existing)"
    echo "================================================================"
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify
    echo ""
else
    echo "================================================================"
    echo " Step 1/5: Prepare 300k+ Training Data (.ftmb) — 50/50 mix"
    echo " Started: $(date)"
    echo "================================================================"

    # Dry run first to verify volume
    echo "[Data] Dry run..."
    python3 scripts/prepare_multibranch_data.py \
        --dry-run \
        --samples-per-type 1200 \
        --synthetic-columns 1200 \
        --ratio-distilled 0.5 \
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
        --seed 42 \
        --workers 8

    echo ""
    echo "[Data] Verifying output..."
    python3 scripts/read_ftmb.py "$FTMB_FILE" --stats --verify

    echo ""
    echo "[Data] Complete: $(date)"
fi
echo ""

# --- Step 1b: Create no-header ablation data -------------------------
# Zero out embed (header) features for AC-7 ablation comparison.
# Reads the FTMB file and writes a copy with all embed_features set to 0.

if [[ "$FLAT_ONLY" == "false" ]] && [[ "$SKIP_ABLATION" == "false" ]]; then
    if [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_NOHEADER" ]]; then
        echo "[Data] No-header ablation data exists, reusing"
    else
        echo "[Data] Creating no-header ablation FTMB (zeroing embed features)..."
        python3 -c "
import struct, sys

MAGIC = b'FTMB'
src, dst = sys.argv[1], sys.argv[2]

with open(src, 'rb') as fin, open(dst, 'wb') as fout:
    # Copy header verbatim
    header = fin.read(24)
    fout.write(header)

    version = struct.unpack_from('<I', header, 4)[0]
    n_records = struct.unpack_from('<Q', header, 8)[0]
    char_dim = struct.unpack_from('<H', header, 16)[0]
    embed_dim = struct.unpack_from('<H', header, 18)[0]
    stats_dim = struct.unpack_from('<H', header, 20)[0]

    label_len_size = 4 if version >= 2 else 2
    zero_embed = b'\x00' * (embed_dim * 4)

    for i in range(n_records):
        # Read label
        if version >= 2:
            (llen,) = struct.unpack('<I', fin.read(4))
            label = fin.read(llen)
            fout.write(struct.pack('<I', llen))
        else:
            (llen,) = struct.unpack('<H', fin.read(2))
            label = fin.read(llen)
            fout.write(struct.pack('<H', llen))
        fout.write(label)

        # Copy char features
        char_bytes = fin.read(char_dim * 4)
        fout.write(char_bytes)

        # Zero out embed features
        fin.read(embed_dim * 4)  # skip original
        fout.write(zero_embed)

        # Copy stats features
        stats_bytes = fin.read(stats_dim * 4)
        fout.write(stats_bytes)

    print(f'Wrote {n_records} records to {dst} with zeroed embed features')
" "$FTMB_FILE" "$FTMB_NOHEADER"

        echo "[Data] Verifying no-header ablation data..."
        python3 scripts/read_ftmb.py "$FTMB_NOHEADER" --stats --verify
    fi
    echo ""
fi

# --- Step 2: Train Flat Multi-Branch (M-6) ---------------------------

echo "================================================================"
echo " Step 2/5: Train Flat Multi-Branch (20 epochs)"
echo " Started: $(date)"
echo "================================================================"

FLAT_MODEL="models/sherlock-v3-flat"

if [[ -f "$FLAT_MODEL/model.safetensors" ]]; then
    echo "[Flat] Model already exists, skipping training"
else
    cargo run --bin finetype --no-default-features --features metal --release -- \
        train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$FLAT_MODEL" \
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
echo "Flat training complete: $(date)"
echo ""

# --- Step 3: Train Hierarchical Multi-Branch -------------------------

HIER_MODEL="models/sherlock-v3-hier"

if [[ "$FLAT_ONLY" == "false" ]]; then
    echo "================================================================"
    echo " Step 3/5: Train Hierarchical Multi-Branch (20 epochs)"
    echo " Started: $(date)"
    echo "================================================================"

    if [[ -f "$HIER_MODEL/model.safetensors" ]]; then
        echo "[Hier] Model already exists, skipping training"
    else
        cargo run --bin finetype --no-default-features --features metal --release -- \
            train-multi-branch \
            --data "$FTMB_FILE" \
            --output "$HIER_MODEL" \
            --epochs 20 \
            --batch-size 32 \
            --lr 0.0001 \
            --weight-decay 0.0001 \
            --dropout 0.35 \
            --seed 42 \
            --head hierarchical \
            --patience 10 \
            --no-tui \
        2>&1
    fi

    echo ""
    echo "Hierarchical training complete: $(date)"
    echo ""
else
    echo "[Step 3/5] Hierarchical training — SKIPPED (--flat-only)"
    echo ""
fi

# --- Step 4: Train No-Header Ablation (AC-7) -------------------------

NOHEADER_MODEL="models/sherlock-v3-noheader"

if [[ "$FLAT_ONLY" == "false" ]] && [[ "$SKIP_ABLATION" == "false" ]]; then
    echo "================================================================"
    echo " Step 4/5: Train No-Header Ablation (20 epochs, embed zeroed)"
    echo " Started: $(date)"
    echo " Purpose: AC-7 — measure header contribution to accuracy"
    echo "================================================================"

    if [[ -f "$NOHEADER_MODEL/model.safetensors" ]]; then
        echo "[NoHeader] Model already exists, skipping training"
    else
        cargo run --bin finetype --no-default-features --features metal --release -- \
            train-multi-branch \
            --data "$FTMB_NOHEADER" \
            --output "$NOHEADER_MODEL" \
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
    echo "No-header ablation training complete: $(date)"
    echo ""
else
    echo "[Step 4/5] No-header ablation — SKIPPED (--flat-only or --skip-ablation)"
    echo ""
fi

# --- Step 5: Evaluation -----------------------------------------------

echo "================================================================"
echo " Step 5/5: Evaluation"
echo " Started: $(date)"
echo "================================================================"
echo ""

# Build eval list: always include flat, conditionally add hier + noheader
MODELS_TO_EVAL=("$FLAT_MODEL")
[[ "$FLAT_ONLY" == "false" ]] && MODELS_TO_EVAL+=("$HIER_MODEL")
[[ "$FLAT_ONLY" == "false" ]] && [[ "$SKIP_ABLATION" == "false" ]] && MODELS_TO_EVAL+=("$NOHEADER_MODEL")

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

if [[ "$FLAT_ONLY" == "false" ]] && [[ "$SKIP_ABLATION" == "false" ]]; then
    FLAT_JSON="$FLAT_MODEL/eval/profile_results.json"
    NOHEADER_JSON="$NOHEADER_MODEL/eval/profile_results.json"

    if [[ -f "$FLAT_JSON" ]] && [[ -f "$NOHEADER_JSON" ]]; then
        echo "================================================================"
        echo " Header Ablation Comparison (AC-7)"
        echo "================================================================"
        duckdb -c "
            WITH flat AS (
                SELECT label_correct, label_total, label_accuracy_pct,
                       domain_correct, domain_total, domain_accuracy_pct
                FROM read_json_auto('$FLAT_JSON')
            ),
            noheader AS (
                SELECT label_correct, label_total, label_accuracy_pct,
                       domain_correct, domain_total, domain_accuracy_pct
                FROM read_json_auto('$NOHEADER_JSON')
            )
            SELECT
                'With headers:    ' || flat.label_correct || '/' || flat.label_total
                    || ' (' || flat.label_accuracy_pct || '% label), '
                    || flat.domain_correct || '/' || flat.domain_total
                    || ' (' || flat.domain_accuracy_pct || '% domain)' AS result
            FROM flat
            UNION ALL
            SELECT
                'Without headers: ' || noheader.label_correct || '/' || noheader.label_total
                    || ' (' || noheader.label_accuracy_pct || '% label), '
                    || noheader.domain_correct || '/' || noheader.domain_total
                    || ' (' || noheader.domain_accuracy_pct || '% domain)'
            FROM noheader
            UNION ALL
            SELECT
                'Delta:           ' || (flat.label_accuracy_pct - noheader.label_accuracy_pct)
                    || ' pp label, '
                    || (flat.domain_accuracy_pct - noheader.domain_accuracy_pct)
                    || ' pp domain'
            FROM flat, noheader
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
echo "  1. Compare flat vs hier vs noheader accuracy"
echo "  2. If header delta > 5pp: headers are load-bearing, keep header branch"
echo "  3. If header delta < 2pp: headers add noise, consider removing"
echo "  4. Review findings before merging"
