#!/usr/bin/env bash
# Re-run the ac-04 gold-anchor kill switch for a given frozen head.
# usage: rescore_gold.sh <head-dir> <model-name> <out-dir>
set -e
HEAD=${1:?head dir}; NAME=${2:?model name}; OUT=${3:?out dir}
mkdir -p "$OUT"
python3 scripts/fusion/fusion_predict.py \
  --input output/late-fusion/gold_anchor/candidate_input.csv \
  --head "$HEAD" \
  --out "$OUT/preds_keyed.tsv" \
  --workdir "$OUT/predict_tmp"
python3 - "$OUT/preds_keyed.tsv" "$OUT/predictions.tsv" <<'PY'
import csv, sys
inp, out = sys.argv[1], sys.argv[2]
with open(inp) as f, open(out, "w", newline="") as o:
    w = csv.writer(o, delimiter="\t")
    w.writerow(["file_content_sha256", "column_name", "predicted_label"])
    for line in f:
        key, pred = line.rstrip("\n").split("\t", 1)
        w.writerow([key[:64], key[64:], pred])
PY
python3 scripts/score_gold_anchor.py score \
  --gold eval/gold/gold_eval_anchor.tsv \
  --predictions "$OUT/predictions.tsv" \
  --model-name "$NAME" --out-dir "$OUT"
