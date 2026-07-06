#!/usr/bin/env bash
# Trim-round re-gate: both passes under the range-validated binary, then the gate
# and same-day gold/repr for both models (bar re-presentation evidence).
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 PYTHONUNBUFFERED=1
PY="eval/gittables/.venv/bin/python"

for M in m2v8m-s43 attneg2-s44; do
  echo "== pass: $M =="
  rm -rf "output/attneg-retrain/trim_${M}_pass"
  FINETYPE_MODEL="models/$M" "$PY" scripts/gittables_corpus_pass.py \
    --corpus-index output/corpus-honest-gate/stratified_sample.files.txt \
    --finetype-bin ./target/release/finetype --execute --jobs 8 \
    --out-dir "output/attneg-retrain/trim_${M}_pass"
done

echo "== oracle join =="
duckdb -c "COPY (SELECT b.file_path, b.column_name, b.sense_prediction, o.ydf_prediction_gated
  FROM read_parquet('output/attneg-retrain/trim_m2v8m-s43_pass/corpus_pass/columns.parquet') b
  LEFT JOIN read_parquet('output/ydf-validation-gate/v19_gated.parquet') o USING (file_path, column_name))
  TO 'output/attneg-retrain/trim_baseline_with_oracle.parquet' (FORMAT parquet)"

echo "== gate =="
"$PY" scripts/corpus_honest_gate.py \
  --baseline output/attneg-retrain/trim_baseline_with_oracle.parquet \
  --candidate output/attneg-retrain/trim_attneg2-s44_pass/corpus_pass/columns.parquet \
  --sample output/corpus-honest-gate/stratified_sample.files.txt \
  --resolution output/corpus-honest-gate/stratified_sample.resolution.json \
  --label attneg2-trim --out-dir output/attneg-retrain || true

echo "== gold + repr, both models, new binary =="
for M in m2v8m-s43 attneg2-s44; do
  FINETYPE_MODEL="models/$M" "$PY" scripts/score_gold_anchor.py predict \
    --gold eval/gold/gold_corpus.tsv --columns eval/gittables/corpus_pass/columns.parquet \
    --binary ./target/release/finetype --out "output/attneg-retrain/trim_predictions_$M.tsv"
  "$PY" scripts/score_gold_anchor.py score --gold eval/gold/gold_corpus.tsv \
    --predictions "output/attneg-retrain/trim_predictions_$M.tsv" --reframe \
    --model-name "trim-$M" --out-dir output/attneg-retrain
  FINETYPE_MODEL="models/$M" "$PY" scripts/score_gold_anchor.py predict \
    --gold eval/repr/representative_corpus.tsv --columns eval/gittables/corpus_pass/columns.parquet \
    --binary ./target/release/finetype --out "output/attneg-retrain/trim_predictions_${M}_repr.tsv"
  "$PY" scripts/score_gold_anchor.py score --gold eval/repr/representative_corpus.tsv \
    --predictions "output/attneg-retrain/trim_predictions_${M}_repr.tsv" --reframe \
    --model-name "trim-$M-repr" --out-dir output/attneg-retrain
done
echo "TRIM_REGATE_DONE"
