#!/usr/bin/env bash
# MOVE 3a: raw-Sense logit-adjust A/B. Read-only wrt tracked source; writes only under output/logit-adjust-ab/.
set -eo pipefail
cd "$(cd "$(dirname "$0")/../.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 HF_HUB_DISABLE_TELEMETRY=1 PYTHONUNBUFFERED=1
PM=./target/release/predict_multibranch
PY=eval/gittables/.venv/bin/python
MODEL=models/m2v8m-s43
GOLDFTMB=output/embed-frontier/gold_m2v8m.ftmb
GOLD=eval/gold/gold_corpus.tsv
OUT=output/logit-adjust-ab
mkdir -p "$OUT/reports"

raw_one () {  # tag  extra-args...
  local tag="$1"; shift
  local raw="$OUT/${tag}_raw.tsv" sense="$OUT/${tag}_sense.tsv"
  "$PM" --model "$MODEL" --data "$GOLDFTMB" --out "$raw" "$@" 2>"$OUT/reports/${tag}_predict.log"
  { echo -e "file_content_sha256\tcolumn_name\tpredicted_label\tconfidence";
    tail -n +2 "$raw" | awk -F'\t' '{split($1,a,"\037"); print a[1]"\t"a[2]"\t"$2"\t"$3}'; } > "$sense"
  mkdir -p "$OUT/reports/${tag}_sense"
  "$PY" scripts/score_gold_anchor.py score --gold "$GOLD" --predictions "$sense" \
     --model-name "${tag}-sense" --out-dir "$OUT/reports/${tag}_sense" >/dev/null 2>&1 || true
  local hs
  hs=$(grep -hiE "Headline" "$OUT/reports/${tag}_sense"/report_*.md 2>/dev/null | head -1)
  echo "${tag}  ${hs}"
}

raw_one tau00
for tau in 0.5 0.75 1.0; do
  tag="tau$(echo $tau | tr -d '.')_train"
  raw_one "$tag" --logit-adjust "$tau" --priors "$OUT/priors_train.tsv"
done
echo "DONE-RAW"
