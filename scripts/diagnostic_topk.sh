#!/usr/bin/env bash
# diagnostic_topk.sh — raw softmax top-5 + full-pipeline prediction for
# container-type and datetime-subtype collapse clusters.
#
# Answers the key question: is the correct label in the model's top-5
# (hint-layer problem) or absent (model representation gap)?
#
# Usage: ./scripts/diagnostic_topk.sh
# Output: diagnostics/container_datetime_topk.tsv
#         diagnostics/container_datetime_pipeline.tsv

set -eo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CSV="$REPO_ROOT/eval/datasets/csv/coverage_closure_phase_ab.csv"
MODEL="$REPO_ROOT/models/default"
K=5
OUT_DIR="$REPO_ROOT/diagnostics"

mkdir -p "$OUT_DIR"

# Build the topk example once
echo "Building amvg_topk example..."
cargo build --example amvg_topk -p finetype-model --release 2>/dev/null

TOPK_FILE="$OUT_DIR/container_datetime_topk.tsv"
PIPELINE_FILE="$OUT_DIR/container_datetime_pipeline.tsv"

# Header for topk output
printf "cluster\tcolumn\tgt_label\trank\tpredicted_label\tprobability\tgt_in_topk\n" > "$TOPK_FILE"

# Header for pipeline output
printf "cluster\tcolumn\tgt_label\tpipeline_prediction\tmatch\n" > "$PIPELINE_FILE"

# Column definitions: cluster|column|gt_label
COLUMNS=(
  "container|csv|container.object.csv"
  "container|html|container.object.html"
  "container|json_array|container.object.json_array"
  "container|query_string|container.key_value.query_string"
  "container|semicolon_separated|container.array.semicolon_separated"
  "container|whitespace_separated|container.array.whitespace_separated"
  "container|xml|container.object.xml"
  "container|yaml|container.object.yaml"
  "datetime|iso_8601_compact|datetime.timestamp.iso_8601_compact"
  "datetime|iso_8601_milliseconds|datetime.timestamp.iso_8601_milliseconds"
  "datetime|iso_microseconds|datetime.timestamp.iso_microseconds"
  "datetime|jp_era_short|datetime.date.jp_era_short"
  "datetime|pg_short_offset|datetime.timestamp.pg_short_offset"
  "datetime|ordinal|datetime.date.ordinal"
)

for entry in "${COLUMNS[@]}"; do
  IFS='|' read -r cluster col gt <<< "$entry"

  echo "  $cluster / $col (gt: $gt)"

  # Raw softmax top-5
  topk_output=$(cargo run --example amvg_topk --release -p finetype-model -- "$CSV" "$col" "$MODEL" "$K" 2>/dev/null) || true

  # Check if GT label appears in top-5
  gt_found="NO"
  if echo "$topk_output" | grep -qF "$gt"; then
    gt_found="YES"
  fi

  while IFS=$'\t' read -r rank label prob; do
    printf "%s\t%s\t%s\t%s\t%s\t%s\t%s\n" "$cluster" "$col" "$gt" "$rank" "$label" "$prob" "$gt_found" >> "$TOPK_FILE"
  done <<< "$topk_output"

  # Full pipeline prediction (with Sharpen)
  pipeline_output=$(cargo run --release -- profile "$CSV" --columns "$col" -o csv 2>/dev/null | tail -1 | cut -d',' -f2) || true

  match="NO"
  if [[ "$pipeline_output" == "$gt" ]]; then
    match="YES"
  fi
  printf "%s\t%s\t%s\t%s\t%s\n" "$cluster" "$col" "$gt" "$pipeline_output" "$match" >> "$PIPELINE_FILE"
done

echo ""
echo "=== Results ==="
echo ""
echo "--- Raw softmax top-5 (pre-Sharpen) ---"
column -t -s$'\t' "$TOPK_FILE"
echo ""
echo "--- Full pipeline (post-Sharpen) ---"
column -t -s$'\t' "$PIPELINE_FILE"
echo ""

# Summary
total=14
gt_in_topk=$(tail -n +2 "$TOPK_FILE" | awk -F'\t' '!seen[$2]++ && $7=="YES"' | wc -l | tr -d ' ')
pipeline_correct=$(tail -n +2 "$PIPELINE_FILE" | awk -F'\t' '$5=="YES"' | wc -l | tr -d ' ')

echo "Summary: $gt_in_topk / $total have GT in raw top-5 (hint-layer candidates)"
echo "         $pipeline_correct / $total correct after full pipeline"
echo ""
echo "Artefacts:"
echo "  $TOPK_FILE"
echo "  $PIPELINE_FILE"
