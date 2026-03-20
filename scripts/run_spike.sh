#!/usr/bin/env bash
# scripts/run_spike.sh — Run all retraining spike experiments end-to-end
#
# Trains each data mix, scores against Tier 1 (profile eval) and Tier 2
# (benchmark), and logs results to a comparison CSV.
#
# Usage:
#   ./scripts/run_spike.sh [OPTIONS]
#
# Options:
#   --training-dir DIR    Directory with NDJSON training files (default: output/spike-training)
#   --results-dir DIR     Directory for results (default: output/spike-results)
#   --skip-existing       Skip experiments that already have results
#   --only EXPERIMENT     Run only this experiment (e.g. "blend-70-30")
#   --help                Show this help
set -euo pipefail

# ─── Defaults ───────────────────────────────────────────────────────
TRAINING_DIR="output/spike-training"
RESULTS_DIR="output/spike-results"
SKIP_EXISTING=false
ONLY=""
SYMLINK_PATH="models/default"
PRODUCTION_MODEL="char-cnn-v14-250"

# ─── Usage ──────────────────────────────────────────────────────────
usage() {
    cat <<'USAGE'
Usage: scripts/run_spike.sh [OPTIONS]

Run all retraining spike experiments: train → Tier 2 score → Tier 1 eval → log.

Options:
  --training-dir DIR    Directory with NDJSON training files (default: output/spike-training)
  --results-dir DIR     Directory for results (default: output/spike-results)
  --skip-existing       Skip experiments that already have results
  --only EXPERIMENT     Run only this experiment (e.g. "blend-70-30")
  --help                Show this help
USAGE
    exit 0
}

# ─── Parse arguments ────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --training-dir) TRAINING_DIR="$2"; shift 2 ;;
        --results-dir)  RESULTS_DIR="$2";  shift 2 ;;
        --skip-existing) SKIP_EXISTING=true; shift ;;
        --only)         ONLY="$2";         shift 2 ;;
        --help|-h)      usage ;;
        *)              echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# ─── Pre-flight checks ─────────────────────────────────────────────
assert_production_symlink() {
    if [[ ! -L "$SYMLINK_PATH" ]]; then
        echo "FATAL: $SYMLINK_PATH is not a symlink" >&2
        exit 1
    fi
    local target
    target="$(readlink "$SYMLINK_PATH")"
    if [[ "$target" != "$PRODUCTION_MODEL" ]]; then
        echo "FATAL: $SYMLINK_PATH points to '$target', expected '$PRODUCTION_MODEL'" >&2
        echo "Restore with: ln -sfn $PRODUCTION_MODEL $SYMLINK_PATH" >&2
        exit 1
    fi
}

restore_symlink() {
    rm -f "$SYMLINK_PATH"
    ln -s "$PRODUCTION_MODEL" "$SYMLINK_PATH"
}

# Trap: always restore production symlink on exit
trap 'restore_symlink; echo ""; echo "Restored $SYMLINK_PATH -> $PRODUCTION_MODEL"' EXIT

echo "═══════════════════════════════════════════════"
echo "  Retraining Spike — Experiment Runner"
echo "═══════════════════════════════════════════════"
echo ""

# Verify starting state
assert_production_symlink
echo "✓ Pre-flight: $SYMLINK_PATH -> $PRODUCTION_MODEL"

# Check training data exists
if [[ ! -d "$TRAINING_DIR" ]]; then
    echo "Error: Training directory not found: $TRAINING_DIR" >&2
    echo "Run scripts/prepare_spike_data.py first." >&2
    exit 1
fi

mkdir -p "$RESULTS_DIR"

# ─── Comparison CSV header ──────────────────────────────────────────
COMPARISON_CSV="$RESULTS_DIR/comparison.csv"
if [[ ! -f "$COMPARISON_CSV" ]]; then
    echo "mix,n_samples,n_types,tier1_label,tier1_domain,tier2_overall,tier2_agreement,tier2_disagreement,tier2_synthetic,training_time_sec" > "$COMPARISON_CSV"
fi

# ─── Extract Tier 1 score from profile eval output ──────────────────
extract_tier1() {
    # Runs make eval-profile and captures output
    local output_file
    output_file="$(mktemp)"
    make eval-profile 2>&1 | tee "$output_file" || true

    # Parse the DuckDB table output for headline accuracy row:
    # │ Format-detectable (direct + close) │ 190 │ 180 │ 94.7 │ 189 │ 99.5 │
    python3 -c "
import re

with open('$output_file') as f:
    output = f.read()

label_correct = 0
label_total = 0
domain_correct = 0
domain_total = 0

for line in output.splitlines():
    if 'direct + close' in line.lower() or ('format-detectable' in line.lower() and '│' in line):
        # Extract all integers from the pipe-separated row
        nums = re.findall(r'\b(\d+)\b', line)
        if len(nums) >= 3:
            label_total = int(nums[0])
            label_correct = int(nums[1])
        if len(nums) >= 4:
            domain_correct = int(nums[3])
        break

print(f'{label_correct}/{label_total} {domain_correct}/{label_total}')
"
    rm -f "$output_file"
}

# ─── Run one experiment ─────────────────────────────────────────────
run_experiment() {
    local ndjson_file="$1"
    local mix_name
    mix_name="$(basename "$ndjson_file" .ndjson)"
    local model_name="spike-${mix_name}"
    local model_dir="models/${model_name}"

    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  Experiment: ${mix_name}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Check if already done
    if [[ "$SKIP_EXISTING" == true ]] && grep -q "^${mix_name}," "$COMPARISON_CSV" 2>/dev/null; then
        echo "  Skipping (already in comparison.csv)"
        return 0
    fi

    # Count samples and types
    local n_samples n_types
    n_samples=$(wc -l < "$ndjson_file")
    n_types=$(python3 -c "
import json, sys
types = set()
with open('$ndjson_file') as f:
    for line in f:
        types.add(json.loads(line)['classification'])
print(len(types))
")

    echo "  Data: ${n_samples} samples, ${n_types} types"

    # ── Step 1: Train ──────────────────────────────────────────────
    echo ""
    echo "  [1/3] Training ${model_name}..."
    local train_start
    train_start=$(date +%s)

    # Restore production symlink before training (train.sh may read it)
    restore_symlink

    ./scripts/train.sh \
        --data "$ndjson_file" \
        --model-name "$model_name" \
        --epochs 10 \
        --seed 42 \
        --size small

    local train_end train_time
    train_end=$(date +%s)
    train_time=$((train_end - train_start))
    echo "  Training time: ${train_time}s"

    # Verify model was created
    if [[ ! -f "${model_dir}/model.safetensors" ]]; then
        echo "  ERROR: Model not created at ${model_dir}" >&2
        return 1
    fi

    # ── Step 2: Swap symlink and score ─────────────────────────────
    echo ""
    echo "  [2/3] Scoring Tier 2..."
    rm -f "$SYMLINK_PATH"
    ln -s "$model_name" "$SYMLINK_PATH"
    echo "  Symlink: $SYMLINK_PATH -> $model_name"

    local tier2_json="$RESULTS_DIR/${mix_name}_tier2.json"
    python3 scripts/score_tier2.py \
        --benchmark eval/tier2_benchmark.csv \
        --finetype ./target/release/finetype \
        --format json \
        --output "$tier2_json"

    # Parse Tier 2 results
    local tier2_overall tier2_agreement tier2_disagreement tier2_synthetic
    tier2_overall=$(python3 -c "import json; d=json.load(open('$tier2_json')); print(f\"{d['overall_correct']}/{d['overall_total']}\")")
    tier2_agreement=$(python3 -c "import json; d=json.load(open('$tier2_json')); a=d.get('by_agreement',{}).get('yes',{}); print(f\"{a['correct']}/{a['total']}\" if a else 'N/A')" 2>/dev/null || echo "N/A")
    tier2_disagreement=$(python3 -c "import json; d=json.load(open('$tier2_json')); a=d.get('by_agreement',{}).get('no',{}); print(f\"{a['correct']}/{a['total']}\" if a else 'N/A')" 2>/dev/null || echo "N/A")
    tier2_synthetic=$(python3 -c "import json; d=json.load(open('$tier2_json')); a=d.get('by_source',{}).get('synthetic',{}); print(f\"{a['correct']}/{a['total']}\" if a else 'N/A')" 2>/dev/null || echo "N/A")

    echo "  Tier 2: overall=${tier2_overall} agreement=${tier2_agreement} disagreement=${tier2_disagreement} synthetic=${tier2_synthetic}"

    # ── Step 3: Score Tier 1 ───────────────────────────────────────
    echo ""
    echo "  [3/3] Scoring Tier 1 (profile eval)..."
    local tier1_scores
    tier1_scores=$(extract_tier1)
    local tier1_label tier1_domain
    tier1_label=$(echo "$tier1_scores" | awk '{print $1}')
    tier1_domain=$(echo "$tier1_scores" | awk '{print $2}')
    echo "  Tier 1: label=${tier1_label} domain=${tier1_domain}"

    # ── Restore and log ────────────────────────────────────────────
    restore_symlink
    echo "  Restored: $SYMLINK_PATH -> $PRODUCTION_MODEL"

    # Append to comparison CSV
    echo "${mix_name},${n_samples},${n_types},${tier1_label},${tier1_domain},${tier2_overall},${tier2_agreement},${tier2_disagreement},${tier2_synthetic},${train_time}" >> "$COMPARISON_CSV"

    echo ""
    echo "  ✓ ${mix_name} complete"
}

# ─── Main loop ──────────────────────────────────────────────────────
EXPERIMENTS=(
    "synthetic"
    "distilled-backfill"
    "blend-50-50"
    "blend-70-30"
    "blend-30-70"
    "blend-70-30-no-coltype"
)

for exp in "${EXPERIMENTS[@]}"; do
    ndjson_file="${TRAINING_DIR}/${exp}.ndjson"

    # Filter if --only specified
    if [[ -n "$ONLY" ]] && [[ "$exp" != "$ONLY" ]]; then
        continue
    fi

    if [[ ! -f "$ndjson_file" ]]; then
        echo "WARNING: ${ndjson_file} not found, skipping ${exp}" >&2
        continue
    fi

    run_experiment "$ndjson_file"
done

# ─── Post-flight ────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════"
echo "  All experiments complete"
echo "═══════════════════════════════════════════════"
echo ""
echo "Results: ${COMPARISON_CSV}"
echo ""

# Print comparison table
if command -v column &>/dev/null; then
    column -t -s',' "$COMPARISON_CSV"
else
    cat "$COMPARISON_CSV"
fi

# Final assertion
restore_symlink
assert_production_symlink
echo ""
echo "✓ Post-flight: $SYMLINK_PATH -> $PRODUCTION_MODEL"
