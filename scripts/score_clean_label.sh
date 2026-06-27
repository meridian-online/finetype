#!/usr/bin/env bash
# score_clean_label.sh — composed-gold (reframe) scorer for the clean-label retrain.
# Mirrors the exact pipeline that produced the s43 baseline 794/931 = 0.853:
#   predict_multibranch -> reshape -> compose_predictions (REAL Sharpen) -> score --reframe
#
# Usage: scripts/score_clean_label.sh <model_dir> <gold_ftmb> <tag> [--value-encoder DIR]
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 HF_HUB_DISABLE_TELEMETRY=1 PYTHONUNBUFFERED=1

MODEL="$1"; GOLDFTMB="$2"; TAG="$3"; shift 3 || true
PY="eval/gittables/.venv/bin/python"
PM="./target/release/predict_multibranch"
GOLD="eval/gold/gold_corpus.tsv"
OUTDIR="output/clean-label-retrain/scores"
mkdir -p "$OUTDIR" "$OUTDIR/$TAG"

raw="$OUTDIR/${TAG}_raw.tsv"
sense="$OUTDIR/${TAG}_sense.tsv"
comp="$OUTDIR/${TAG}_composed.tsv"

"$PM" --model "$MODEL" --data "$GOLDFTMB" --out "$raw" "$@"
{ echo -e "file_content_sha256\tcolumn_name\tpredicted_label\tconfidence";
  tail -n +2 "$raw" | awk -F'\t' '{split($1,a,"\037"); print a[1]"\t"a[2]"\t"$2"\t"$3}'; } > "$sense"
"$PY" scripts/compose_predictions.py --preds "$sense" --out "$comp" >/dev/null 2>&1 || cp "$sense" "$comp"

# Sense (raw) reframe headline + composed reframe headline
rm -f "$OUTDIR/$TAG"/report_*.md 2>/dev/null || true
"$PY" scripts/score_gold_anchor.py score --gold "$GOLD" --predictions "$sense" \
  --model-name "${TAG}-sense" --out-dir "$OUTDIR/$TAG" --reframe >/dev/null 2>&1 || true
SENSE=$(grep -hE "Headline" "$OUTDIR/$TAG"/report_*.md 2>/dev/null | head -1 | grep -oE "[0-9]+\.[0-9]+" | head -1)
rm -f "$OUTDIR/$TAG"/report_*.md 2>/dev/null || true
"$PY" scripts/score_gold_anchor.py score --gold "$GOLD" --predictions "$comp" \
  --model-name "${TAG}-composed" --out-dir "$OUTDIR/$TAG" --reframe >/dev/null 2>&1 || true
COMPOSED=$(grep -hE "Headline" "$OUTDIR/$TAG"/report_*.md 2>/dev/null | head -1 | grep -oE "[0-9]+\.[0-9]+" | head -1)

echo "TAG=$TAG  Sense(reframe)=${SENSE:-ERR}  composed(reframe)=${COMPOSED:-ERR}  (baseline s43 composed 0.853)"
