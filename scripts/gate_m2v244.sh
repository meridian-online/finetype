#!/usr/bin/env bash
# scripts/gate_m2v244.sh — corpus-honest gate (H05, BLOCKING) for m2v-244 vs the current default.
#
# m2v-244 (potion-4M, 244-label) is native-runnable, so we score it directly with `profile`
# over the ac-01 stratified sample, then judge transitions against the STANDING CURRENT-DEFAULT
# reference pass (output/corpus-honest-gate/standing/baseline_m2v8m-s43.parquet — the shipped
# default's predictions + the column-intrinsic gated-YDF oracle). Since de-fossilisation
# (choice 0104) --baseline is REQUIRED and must NOT be the retired v19_gated oracle (that scored
# deviation-from-v19). Build the reference once with scripts/build_standing_baseline.sh.
# This is the one blocking gate every ship path needs; a NO-GO blocks the swap.
#
# label-remap: categorical was retired -> word (choice 0102), so the oracle's categorical must
# speak the candidate's vocabulary, matching the documented enum-reframe handling.
#
# Do NOT rebuild ./target/release/finetype while this runs (in-flight subprocesses).
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"

BIN="./target/release/finetype"
MODEL="models/m2v-244-s44"
SAMPLE="output/corpus-honest-gate/stratified_sample.files.txt"
BASELINE="output/corpus-honest-gate/standing/baseline_m2v8m-s43.parquet"
OUT="output/corpus-honest-gate/m2v-244"
LOG="$OUT/run.log"
mkdir -p "$OUT"
exec > >(tee -a "$LOG") 2>&1

echo "================ CORPUS-HONEST GATE: m2v-244 — $(date) ================"
[[ -x "$BIN" ]] || { echo "FAIL: $BIN missing (do NOT rebuild while a gate runs)"; exit 1; }
[[ -f "$MODEL/model.safetensors" ]] || { echo "FAIL: $MODEL missing"; exit 1; }
[[ -f "$SAMPLE" ]] || { echo "FAIL: stratified sample $SAMPLE missing"; exit 1; }
[[ -f "$BASELINE" ]] || { echo "FAIL: standing baseline $BASELINE missing — build it once with scripts/build_standing_baseline.sh"; exit 1; }

# shellcheck disable=SC1091
source eval/gittables/.venv/bin/activate   # pyarrow/duckdb

echo "── Candidate corpus pass (m2v-244-s44) — $(date) ──"
rm -rf "$OUT/candidate"
FINETYPE_MODEL="$MODEL" python3 scripts/gittables_corpus_pass.py \
    --corpus-index "$SAMPLE" --execute --jobs 8 \
    --finetype-bin "$BIN" --out-dir "$OUT/candidate"

CAND_PARQUET="$OUT/candidate/corpus_pass/columns.parquet"
[[ -f "$CAND_PARQUET" ]] || { echo "FAIL: candidate pass produced no parquet"; exit 1; }

echo ""; echo "── Corpus-honest gate (candidate vs standing current-default reference) — $(date) ──"
python3 scripts/corpus_honest_gate.py \
    --baseline "$BASELINE" \
    --candidate "$CAND_PARQUET" \
    --label-remap representation.discrete.categorical=representation.text.word \
    --label "m2v-244" \
    --out-dir "$OUT" | tee "$OUT/corpus_honest_gate.txt"

echo ""; echo "================ DONE — $(date) ================"
echo "verdict: $OUT/corpus_honest_gate.txt"
