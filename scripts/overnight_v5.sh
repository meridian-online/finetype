#!/usr/bin/env bash
# scripts/overnight_v5.sh — Overnight training pipeline v5
#
# Improved data quality + architecture scale-up experiment.
# Spec: specs/2026-03-25-overnight-training-v5/spec.yaml
#
# Pipeline:
#   0. Preflight: Metal GPU probe + timing probe (adaptive epochs)
#   1. Prepare training data (augmented, label-validated, oversampled)
#   2. Train current architecture → sherlock-v5-current
#   3. Train scaled architecture → sherlock-v5-scaled
#   4. Evaluate both (raw + post-Sharpen)
#   5. Per-column regression check + comparison report
#
# Stage markers in OUTPUT_DIR/ enable resume after partial failure.
#
# Usage:
#   ./scripts/overnight_v5.sh                    # Full pipeline
#   ./scripts/overnight_v5.sh --skip-data        # Reuse existing .ftmb
#   ./scripts/overnight_v5.sh --skip-scaled      # Skip scaled architecture
#   ./scripts/overnight_v5.sh --tui              # Show training TUI (interactive)
#   ./scripts/overnight_v5.sh --epochs 40        # Override epoch count
#   ./scripts/overnight_v5.sh --samples 3000     # Override samples per type
#
# Output:
#   output/overnight-v5/                         — All artifacts
#   models/sherlock-v5-current/                  — Current arch model
#   models/sherlock-v5-scaled/                   — Scaled arch model
#   output/overnight-v5/comparison.md            — Final comparison report
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

OUTPUT_DIR="output/overnight-v5"
LOG_FILE="results/overnight-v5.log"
FTMB_FILE="$OUTPUT_DIR/blend-v5.ftmb"

# Training defaults
EPOCHS=40
SAMPLES_PER_TYPE=3000
BATCH_SIZE=32
LR=0.0001
WEIGHT_DECAY=0.0001
DROPOUT=0.35
PATIENCE=10
SEED=42
AUGMENTATION_RATE=0.35
MIN_EPOCHS=20
MAX_WALL_HOURS=7

# Oversampled types (top-10 confusion pairs from eval)
OVERSAMPLE_TYPES="geography.transportation.hs_code,representation.numeric.decimal_number,geography.coordinate.latitude,geography.coordinate.longitude,technology.development.version,identity.government.ssn,identity.person.phone_number,identity.commerce.ean,identity.commerce.upc,identity.medical.npi"

SKIP_DATA=false
SKIP_SCALED=false
USE_TUI=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-data)      SKIP_DATA=true; shift ;;
        --skip-scaled)    SKIP_SCALED=true; shift ;;
        --tui)            USE_TUI=true; shift ;;
        --epochs)         EPOCHS="$2"; shift 2 ;;
        --samples)        SAMPLES_PER_TYPE="$2"; shift 2 ;;
        --help|-h)
            sed -n '2,/^set -/p' "$0" | grep '^#' | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# TUI flag for training commands (--no-tui unless --tui was passed)
TUI_FLAG="--no-tui"
if $USE_TUI; then
    TUI_FLAG=""
fi

mkdir -p "$OUTPUT_DIR" results
if ! $USE_TUI; then
    exec > >(tee -a "$LOG_FILE") 2>&1
fi

echo "================================================================"
echo " Multi-Branch Overnight Pipeline v5 (Data Quality + Scale-Up)"
echo " Started: $(date)"
echo " Host: $(hostname) — $(uname -m)"
echo " Config: $SAMPLES_PER_TYPE samples/type, $EPOCHS epochs, seed $SEED"
echo " Augmentation: ${AUGMENTATION_RATE} rate"
echo " Oversample: $(echo $OVERSAMPLE_TYPES | tr ',' '\n' | wc -l | tr -d ' ') types at 3x"
echo "================================================================"
echo ""

PIPELINE_START=$(date +%s)

# Helper: check if stage is already complete
stage_done() {
    [[ -f "$OUTPUT_DIR/.stage-$1-done" ]]
}
mark_done() {
    date > "$OUTPUT_DIR/.stage-$1-done"
    echo "[Stage $1] Marked complete at $(date)"
}

# ═══════════════════════════════════════════════════════════════════════
# Stage 0: Preflight checks
# ═══════════════════════════════════════════════════════════════════════

echo "================================================================"
echo " Stage 0: Preflight Checks"
echo "================================================================"

# Check prerequisites
if [[ ! -f output/distillation-v3/sherlock_distilled.csv.gz ]]; then
    echo "FAIL: Distilled data not found at output/distillation-v3/sherlock_distilled.csv.gz"
    exit 1
fi

if [[ ! -d models/model2vec ]]; then
    echo "FAIL: Model2Vec not found at models/model2vec/"
    exit 1
fi

if [[ ! -f models/sibling-context/model.safetensors ]]; then
    echo "FAIL: Sibling-context model not found at models/sibling-context/model.safetensors"
    exit 1
fi

if ! command -v duckdb >/dev/null 2>&1; then
    echo "FAIL: duckdb CLI not found on PATH"
    exit 1
fi

echo "[Preflight] Building with Metal..."
cargo build --bin finetype --no-default-features --features metal --release 2>&1
echo "[Preflight] Build OK"

# Verify taxonomy check passes (generators must be good after AC-1 fixes)
echo ""
echo "[Preflight] Verifying taxonomy/generator alignment..."
./target/release/finetype check 2>&1 | tail -5
echo ""

echo "[Preflight] All checks passed"
echo ""

# ═══════════════════════════════════════════════════════════════════════
# Stage 1: Prepare Training Data
# ═══════════════════════════════════════════════════════════════════════

if stage_done "1-data" && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Stage 1: Data Prep — SKIPPED (already complete)"
    echo "================================================================"
elif [[ "$SKIP_DATA" == "true" ]] && [[ -f "$FTMB_FILE" ]]; then
    echo "================================================================"
    echo " Stage 1: Data Prep — SKIPPED (--skip-data)"
    echo "================================================================"
else
    echo "================================================================"
    echo " Stage 1: Prepare Training Data"
    echo " $SAMPLES_PER_TYPE samples/type, augmentation rate $AUGMENTATION_RATE"
    echo " Oversample: top-10 confused types at 3x"
    echo " Started: $(date)"
    echo "================================================================"

    python3 scripts/prepare_multibranch_data.py \
        --distilled output/distillation-v3/sherlock_distilled.csv.gz \
        --finetype ./target/release/finetype \
        --output "$FTMB_FILE" \
        --label-remap data/label_remap.json \
        --samples-per-type "$SAMPLES_PER_TYPE" \
        --synthetic-columns "$SAMPLES_PER_TYPE" \
        --ratio-distilled 0.5 \
        --format v3 \
        --seed "$SEED" \
        --workers 8 \
        --augmentation-rate "$AUGMENTATION_RATE" \
        --validate-labels \
        --oversample-types "$OVERSAMPLE_TYPES"

    mark_done "1-data"
fi
echo ""

# ═══════════════════════════════════════════════════════════════════════
# Stage 1.5: Timing Probe (adaptive epoch count)
# ═══════════════════════════════════════════════════════════════════════

if ! stage_done "1.5-timing"; then
    echo "================================================================"
    echo " Stage 1.5: Timing Probe (2 epochs, current arch)"
    echo "================================================================"

    PROBE_DIR="$OUTPUT_DIR/timing-probe"
    mkdir -p "$PROBE_DIR"

    PROBE_START=$(date +%s)
    ./target/release/finetype train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$PROBE_DIR" \
        --epochs 2 \
        --batch-size "$BATCH_SIZE" \
        --lr "$LR" \
        --weight-decay "$WEIGHT_DECAY" \
        --dropout "$DROPOUT" \
        --seed "$SEED" \
        --head flat \
        --patience 999 \
        $TUI_FLAG \
        2>&1
    PROBE_END=$(date +%s)
    PROBE_ELAPSED=$((PROBE_END - PROBE_START))
    SECS_PER_EPOCH=$((PROBE_ELAPSED / 2))

    echo ""
    echo "[Timing] 2 epochs took ${PROBE_ELAPSED}s → ${SECS_PER_EPOCH}s/epoch"

    # Project total time: current arch × EPOCHS + scaled arch × EPOCHS × 2 (wider) + overhead
    CURRENT_PROJECTED=$((SECS_PER_EPOCH * EPOCHS))
    SCALED_PROJECTED=$((SECS_PER_EPOCH * EPOCHS * 2))  # ~2x FLOPs for 50% wider
    TOTAL_PROJECTED=$((CURRENT_PROJECTED + SCALED_PROJECTED + 3600))  # +1hr overhead
    TOTAL_HOURS=$((TOTAL_PROJECTED / 3600))

    echo "[Timing] Projected: current=${CURRENT_PROJECTED}s + scaled=${SCALED_PROJECTED}s + overhead=3600s"
    echo "[Timing] Total projected: ${TOTAL_PROJECTED}s (~${TOTAL_HOURS}h)"

    MAX_WALL_SECS=$((MAX_WALL_HOURS * 3600))
    if [[ $TOTAL_PROJECTED -gt $MAX_WALL_SECS ]]; then
        # Reduce epochs to fit within budget
        # Budget for training: MAX_WALL_SECS - 3600 overhead
        TRAINING_BUDGET=$((MAX_WALL_SECS - 3600))
        # current + scaled = epochs * secs_per_epoch * (1 + 2) = epochs * secs * 3
        ADAPTIVE_EPOCHS=$((TRAINING_BUDGET / (SECS_PER_EPOCH * 3)))

        if [[ $ADAPTIVE_EPOCHS -lt $MIN_EPOCHS ]]; then
            echo "[Timing] WARNING: Even ${MIN_EPOCHS} epochs exceeds ${MAX_WALL_HOURS}h budget"
            echo "[Timing] Proceeding with ${MIN_EPOCHS} epochs (may exceed wall time)"
            EPOCHS=$MIN_EPOCHS
        else
            echo "[Timing] Reducing epochs: $EPOCHS → $ADAPTIVE_EPOCHS (to fit ${MAX_WALL_HOURS}h budget)"
            EPOCHS=$ADAPTIVE_EPOCHS
        fi
    else
        echo "[Timing] Budget OK: ${TOTAL_HOURS}h ≤ ${MAX_WALL_HOURS}h"
    fi

    # Check Metal throughput (rough: if >60s/epoch for 2-epoch probe on ~800k records, likely CPU)
    if [[ $SECS_PER_EPOCH -gt 600 ]]; then
        echo "[Timing] WARNING: ${SECS_PER_EPOCH}s/epoch is slow — Metal may not be active"
        echo "[Timing] Expected <300s/epoch on M1 Pro Metal with ~800k records"
    fi

    # Save timing results
    echo "${SECS_PER_EPOCH}" > "$OUTPUT_DIR/secs_per_epoch.txt"
    echo "${EPOCHS}" > "$OUTPUT_DIR/adaptive_epochs.txt"

    # Clean up probe model
    rm -rf "$PROBE_DIR"

    mark_done "1.5-timing"
else
    EPOCHS=$(cat "$OUTPUT_DIR/adaptive_epochs.txt" 2>/dev/null || echo "$EPOCHS")
    echo "[Timing] Using previously computed epochs: $EPOCHS"
fi
echo ""

# ═══════════════════════════════════════════════════════════════════════
# Stage 2: Train Current Architecture
# ═══════════════════════════════════════════════════════════════════════

CURRENT_MODEL="models/sherlock-v5-current"

if stage_done "2-train-current"; then
    echo "================================================================"
    echo " Stage 2: Train Current Arch — SKIPPED (already complete)"
    echo "================================================================"
else
    echo "================================================================"
    echo " Stage 2: Train Current Architecture ($EPOCHS epochs)"
    echo " Config: default (char 300, embed 200, stats 128→64, trunk 500→500)"
    echo " Started: $(date)"
    echo "================================================================"

    ./target/release/finetype train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$CURRENT_MODEL" \
        --epochs "$EPOCHS" \
        --batch-size "$BATCH_SIZE" \
        --lr "$LR" \
        --weight-decay "$WEIGHT_DECAY" \
        --dropout "$DROPOUT" \
        --seed "$SEED" \
        --head flat \
        --patience "$PATIENCE" \
        $TUI_FLAG \
        2>&1

    echo ""
    echo "Current architecture training complete: $(date)"
    mark_done "2-train-current"
fi
echo ""

# ═══════════════════════════════════════════════════════════════════════
# Stage 3: Train Scaled Architecture
# ═══════════════════════════════════════════════════════════════════════

SCALED_MODEL="models/sherlock-v5-scaled"
SCALED_CONFIG="models/sherlock-v5-scaled-config.json"

if [[ "$SKIP_SCALED" == "true" ]]; then
    echo "================================================================"
    echo " Stage 3: Train Scaled Arch — SKIPPED (--skip-scaled)"
    echo "================================================================"
elif stage_done "3-train-scaled"; then
    echo "================================================================"
    echo " Stage 3: Train Scaled Arch — SKIPPED (already complete)"
    echo "================================================================"
else
    echo "================================================================"
    echo " Stage 3: Train Scaled Architecture ($EPOCHS epochs)"
    echo " Config: scaled (char 450, embed 300, stats 192→96, trunk 750→750)"
    echo " Started: $(date)"
    echo "================================================================"

    if [[ ! -f "$SCALED_CONFIG" ]]; then
        echo "FAIL: Scaled config not found at $SCALED_CONFIG"
        exit 1
    fi

    ./target/release/finetype train-multi-branch \
        --data "$FTMB_FILE" \
        --output "$SCALED_MODEL" \
        --epochs "$EPOCHS" \
        --batch-size "$BATCH_SIZE" \
        --lr "$LR" \
        --weight-decay "$WEIGHT_DECAY" \
        --dropout "$DROPOUT" \
        --seed "$SEED" \
        --head flat \
        --patience "$PATIENCE" \
        $TUI_FLAG \
        --model-config "$SCALED_CONFIG" \
        2>&1

    echo ""
    echo "Scaled architecture training complete: $(date)"
    mark_done "3-train-scaled"
fi
echo ""

# ═══════════════════════════════════════════════════════════════════════
# Stage 4: Evaluation
# ═══════════════════════════════════════════════════════════════════════

echo "================================================================"
echo " Stage 4: Evaluation"
echo " Started: $(date)"
echo "================================================================"
echo ""

MODELS_TO_EVAL=("$CURRENT_MODEL")
[[ "$SKIP_SCALED" == "false" ]] && MODELS_TO_EVAL+=("$SCALED_MODEL")

for model_dir in "${MODELS_TO_EVAL[@]}"; do
    model_name="$(basename "$model_dir")"
    if [[ -f "$model_dir/model.safetensors" ]]; then
        echo "-- Evaluating $model_name --"
        ./scripts/eval.sh --model "$model_dir" || {
            echo "WARN: Eval failed for $model_dir"
            continue
        }

        # Preserve eval results
        mkdir -p "$model_dir/eval"
        cp -r eval/eval_output/* "$model_dir/eval/" 2>/dev/null || true
        echo "  Eval results saved to $model_dir/eval/"
        echo ""
    else
        echo "-- Skipping $model_name (no model.safetensors) --"
        echo ""
    fi
done

# ═══════════════════════════════════════════════════════════════════════
# Stage 5: Comparison Report
# ═══════════════════════════════════════════════════════════════════════

echo "================================================================"
echo " Stage 5: Comparison Report"
echo "================================================================"
echo ""

REPORT="$OUTPUT_DIR/comparison.md"

cat > "$REPORT" <<'HEADER'
# Overnight Training v5 — Comparison Report

**Spec:** specs/2026-03-25-overnight-training-v5/spec.yaml
HEADER

echo "**Date:** $(date)" >> "$REPORT"
echo "**Host:** $(hostname) — $(uname -m)" >> "$REPORT"
echo "" >> "$REPORT"
echo "## Training Configuration" >> "$REPORT"
echo "" >> "$REPORT"
echo "| Parameter | Value |" >> "$REPORT"
echo "|-----------|-------|" >> "$REPORT"
echo "| Samples per type | $SAMPLES_PER_TYPE |" >> "$REPORT"
echo "| Epochs | $EPOCHS |" >> "$REPORT"
echo "| Augmentation rate | $AUGMENTATION_RATE |" >> "$REPORT"
echo "| Batch size | $BATCH_SIZE |" >> "$REPORT"
echo "| Learning rate | $LR |" >> "$REPORT"
echo "| Seed | $SEED |" >> "$REPORT"
echo "" >> "$REPORT"

echo "## Results" >> "$REPORT"
echo "" >> "$REPORT"
echo "| Model | Raw Label | Raw Domain | Post-Sharpen Label | Post-Sharpen Domain |" >> "$REPORT"
echo "|-------|-----------|------------|-------------------|-------------------|" >> "$REPORT"

for model_dir in "${MODELS_TO_EVAL[@]}"; do
    model_name="$(basename "$model_dir")"
    JSON_FILE="$model_dir/eval/profile_results.json"
    if [[ -f "$JSON_FILE" ]]; then
        ROW=$(duckdb -c "SELECT
            '| $model_name | ' ||
            label_correct || '/' || label_total || ' (' || label_accuracy_pct || '%) | ' ||
            domain_correct || '/' || domain_total || ' (' || domain_accuracy_pct || '%) | ' ||
            'TBD | TBD |'
          FROM read_json_auto('$JSON_FILE')" -noheader -csv 2>/dev/null || echo "| $model_name | ERROR | ERROR | TBD | TBD |")
        echo "$ROW" >> "$REPORT"
    fi
done

echo "" >> "$REPORT"
echo "## Baseline Comparison" >> "$REPORT"
echo "" >> "$REPORT"
echo "| Model | Label Accuracy | vs Baseline (155/190) |" >> "$REPORT"
echo "|-------|---------------|----------------------|" >> "$REPORT"
echo "| sherlock-v4-sibling (baseline) | 155/190 (81.6%) | — |" >> "$REPORT"

for model_dir in "${MODELS_TO_EVAL[@]}"; do
    model_name="$(basename "$model_dir")"
    JSON_FILE="$model_dir/eval/profile_results.json"
    if [[ -f "$JSON_FILE" ]]; then
        SCORE=$(duckdb -c "SELECT label_correct FROM read_json_auto('$JSON_FILE')" -noheader -csv 2>/dev/null || echo "?")
        if [[ "$SCORE" =~ ^[0-9]+$ ]]; then
            DELTA=$((SCORE - 155))
            SIGN=""
            [[ $DELTA -gt 0 ]] && SIGN="+"
            echo "| $model_name | ${SCORE}/190 | ${SIGN}${DELTA} |" >> "$REPORT"
        fi
    fi
done

echo "" >> "$REPORT"

# Timing summary
PIPELINE_END=$(date +%s)
TOTAL_ELAPSED=$((PIPELINE_END - PIPELINE_START))
TOTAL_HOURS=$((TOTAL_ELAPSED / 3600))
TOTAL_MINS=$(( (TOTAL_ELAPSED % 3600) / 60))

echo "## Timing" >> "$REPORT"
echo "" >> "$REPORT"
echo "Total pipeline time: ${TOTAL_HOURS}h ${TOTAL_MINS}m" >> "$REPORT"
echo "" >> "$REPORT"

echo "================================================================"
echo " Pipeline Complete"
echo " Finished: $(date)"
echo " Total time: ${TOTAL_HOURS}h ${TOTAL_MINS}m"
echo " Report: $REPORT"
echo " Log: $LOG_FILE"
echo "================================================================"
echo ""

# Print summary
echo "Results:"
for model_dir in "${MODELS_TO_EVAL[@]}"; do
    model_name="$(basename "$model_dir")"
    JSON_FILE="$model_dir/eval/profile_results.json"
    if [[ -f "$JSON_FILE" ]]; then
        echo "  $model_name:"
        duckdb -c "SELECT
            '    Label: ' || label_correct || '/' || label_total || ' (' || label_accuracy_pct || '%)' AS r
          FROM read_json_auto('$JSON_FILE')
          UNION ALL
          SELECT
            '    Domain: ' || domain_correct || '/' || domain_total || ' (' || domain_accuracy_pct || '%)'
          FROM read_json_auto('$JSON_FILE')" -noheader -csv 2>/dev/null || echo "    (failed to read)"
    fi
done
echo ""
echo "Next: Review comparison.md, update models/default symlink, publish to HuggingFace"
