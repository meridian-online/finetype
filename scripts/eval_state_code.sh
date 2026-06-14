#!/usr/bin/env bash
# scripts/eval_state_code.sh — gold + corpus-honest gate for the coordinate
# promote-guard (a Sharpen RULE change, not a model retrain). The candidate is the
# freshly-built binary with the shipped v19 model; the baseline is v19 itself.
set -uo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
OUT="output/state-code-promote"; mkdir -p "$OUT"
BIN="./target/release/finetype"
VPY="eval/gittables/.venv/bin/python"
MODEL="models/default"
GOLD="eval/gold/gold_corpus_v1.tsv"
COLS="eval/gittables/corpus_pass/columns.parquet"
LOG="$OUT/eval.log"; exec > >(tee -a "$LOG") 2>&1

echo "=== coord promote-guard eval $(date) ==="

echo "── [1/3] Gold anchor (new binary + v19) ──"
FINETYPE_MODEL="$MODEL" "$VPY" scripts/score_gold_anchor.py predict \
  --gold "$GOLD" --columns "$COLS" --binary "$BIN" \
  --out "$OUT/predictions_state_code.tsv" || echo "  predict failed"
"$VPY" scripts/score_gold_anchor.py score \
  --gold "$GOLD" --predictions "$OUT/predictions_state_code.tsv" \
  --model-name state-code --out-dir "$OUT" || echo "  score failed"

echo "── [2/3] Candidate corpus pass (33k stratified sample, new binary) ──"
# shellcheck disable=SC1091
source eval/gittables/.venv/bin/activate
FINETYPE_MODEL="$MODEL" python3 scripts/gittables_corpus_pass.py \
  --corpus-index output/corpus-honest-gate/stratified_sample.files.txt \
  --execute --jobs 8 --out-dir "$OUT/sample_pass" || echo "  corpus pass failed"

echo "── [3/3] Corpus-honest gate (BLOCKING) ──"
python3 scripts/corpus_honest_gate.py \
  --candidate "$OUT/sample_pass/corpus_pass/columns.parquet" \
  --label state-code | tee "$OUT/corpus_honest_gate.txt"

echo "=== done $(date) — verdict in $OUT/corpus_honest_gate.txt ==="
