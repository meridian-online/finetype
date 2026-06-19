#!/usr/bin/env bash
# ac-03 eval: predict gold + representative with the datetime-rule binary, score vs v19.
set -euo pipefail
cd "$(dirname "$0")/../.."
BIN=target/release/finetype
PY=eval/gittables/.venv/bin/python
OUT=output/deterministic-datetime
mkdir -p "$OUT"

echo "=== predict GOLD (candidate = v19 + datetime rule) ==="
$PY scripts/score_gold_anchor.py predict \
  --gold eval/gold/gold_corpus.tsv \
  --columns eval/gittables/corpus_pass/columns.parquet \
  --binary "$BIN" --out "$OUT/pred_gold_cand.tsv"

echo "=== score GOLD --reframe ==="
$PY scripts/score_gold_anchor.py score \
  --gold eval/gold/gold_corpus.tsv \
  --predictions "$OUT/pred_gold_cand.tsv" \
  --model-name datetime-rule --out-dir "$OUT/gold" --reframe

echo "=== predict REPRESENTATIVE ==="
$PY scripts/score_gold_anchor.py predict \
  --gold eval/repr/representative_corpus.tsv \
  --columns eval/gittables/corpus_pass/columns.parquet \
  --binary "$BIN" --out "$OUT/pred_repr_cand.tsv"

echo "=== score REPRESENTATIVE --reframe ==="
$PY scripts/score_gold_anchor.py score \
  --gold eval/repr/representative_corpus.tsv \
  --predictions "$OUT/pred_repr_cand.tsv" \
  --model-name datetime-rule --out-dir "$OUT/repr" --reframe

echo "DONE"
