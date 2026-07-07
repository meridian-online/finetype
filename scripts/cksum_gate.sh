#!/usr/bin/env bash
# npi/upc checksum-guard gate: candidate = m2v8m-s43 + Luhn/GS1 guards (no W2.7),
# baseline = the same model WITHOUT guards (preW27 pass = 0.6.40 behaviour). Isolates
# the guards' corpus effect, then same-binary gold + repr. The gate NO-GO on npi/upc
# collapse is the documented checksum-blind false alarm — adjudicated by gold + per-column.
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 PYTHONUNBUFFERED=1
PY="eval/gittables/.venv/bin/python"
M=m2v8m-s43

echo "== candidate pass: $M + checksum guards (no W2.7) =="
rm -rf output/attneg-retrain/cksum_pass
FINETYPE_MODEL=models/$M "$PY" scripts/gittables_corpus_pass.py \
  --corpus-index output/corpus-honest-gate/stratified_sample.files.txt \
  --finetype-bin ./target/release/finetype --execute --jobs 8 \
  --out-dir output/attneg-retrain/cksum_pass

echo "== gate: baseline=0.6.40 (preW27, no guards) vs candidate=guards =="
"$PY" scripts/corpus_honest_gate.py \
  --baseline output/attneg-retrain/preW27_baseline_with_oracle.parquet \
  --candidate output/attneg-retrain/cksum_pass/corpus_pass/columns.parquet \
  --sample output/corpus-honest-gate/stratified_sample.files.txt \
  --resolution output/corpus-honest-gate/stratified_sample.resolution.json \
  --label cksum-guard --out-dir output/attneg-retrain || true

echo "== gold + repr (guard binary) =="
FINETYPE_MODEL=models/$M "$PY" scripts/score_gold_anchor.py predict \
  --gold eval/gold/gold_corpus.tsv --columns eval/gittables/corpus_pass/columns.parquet \
  --binary ./target/release/finetype --out output/attneg-retrain/cksum_predictions_gold.tsv
"$PY" scripts/score_gold_anchor.py score --gold eval/gold/gold_corpus.tsv \
  --predictions output/attneg-retrain/cksum_predictions_gold.tsv --reframe \
  --model-name cksum-guard-gold --out-dir output/attneg-retrain
FINETYPE_MODEL=models/$M "$PY" scripts/score_gold_anchor.py predict \
  --gold eval/repr/representative_corpus.tsv --columns eval/gittables/corpus_pass/columns.parquet \
  --binary ./target/release/finetype --out output/attneg-retrain/cksum_predictions_repr.tsv
"$PY" scripts/score_gold_anchor.py score --gold eval/repr/representative_corpus.tsv \
  --predictions output/attneg-retrain/cksum_predictions_repr.tsv --reframe \
  --model-name cksum-guard-repr --out-dir output/attneg-retrain

echo "CKSUM_GATE_DONE"
