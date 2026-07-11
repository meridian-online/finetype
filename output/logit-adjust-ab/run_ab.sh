#!/usr/bin/env bash
# MOVE 3a logit-adjust A/B driver. Read-only wrt tracked source; writes only under output/logit-adjust-ab/.
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

run_one () {  # tag  extra-args...
  local tag="$1"; shift
  local raw="$OUT/${tag}_raw.tsv" sense="$OUT/${tag}_sense.tsv" comp="$OUT/${tag}_composed.tsv"
  "$PM" --model "$MODEL" --data "$GOLDFTMB" --out "$raw" "$@" 2>"$OUT/reports/${tag}_predict.log"
  { echo -e "file_content_sha256\tcolumn_name\tpredicted_label\tconfidence";
    tail -n +2 "$raw" | awk -F'\t' '{split($1,a,"\037"); print a[1]"\t"a[2]"\t"$2"\t"$3}'; } > "$sense"
  "$PY" scripts/compose_predictions.py --preds "$sense" --out "$comp" >/dev/null 2>&1 || cp "$sense" "$comp"
  mkdir -p "$OUT/reports/${tag}_sense" "$OUT/reports/${tag}_comp"
  "$PY" scripts/score_gold_anchor.py score --gold "$GOLD" --predictions "$sense" \
     --model-name "${tag}-sense" --out-dir "$OUT/reports/${tag}_sense" >/dev/null 2>&1 || true
  "$PY" scripts/score_gold_anchor.py score --gold "$GOLD" --predictions "$comp" \
     --model-name "${tag}-comp" --out-dir "$OUT/reports/${tag}_comp" >/dev/null 2>&1 || true
  local hs hc
  hs=$(grep -hiE "Headline" "$OUT/reports/${tag}_sense"/report_*.md 2>/dev/null | head -1 | grep -oE "[0-9]+/[0-9]+ = [0-9.]+|[0-9]+\.[0-9]+" | head -1)
  hc=$(grep -hiE "Headline" "$OUT/reports/${tag}_comp"/report_*.md  2>/dev/null | head -1 | grep -oE "[0-9]+/[0-9]+ = [0-9.]+|[0-9]+\.[0-9]+" | head -1)
  echo "${tag}  sense=${hs:-ERR}  composed=${hc:-ERR}"
}

echo "== tau=0 (baseline, prior-independent) =="
run_one tau00

for prior in train emission; do
  PF="$OUT/priors_${prior}.tsv"
  for tau in 0.5 0.75 1.0; do
    tag="tau$(echo $tau | tr -d '.')_${prior}"
    echo "== tau=$tau prior=$prior =="
    run_one "$tag" --logit-adjust "$tau" --priors "$PF"
  done
done
echo "DONE"
