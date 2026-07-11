#!/usr/bin/env bash
# Offline corpus-honest gate for potion-8M + code-16M (non-native), then diff triggers.
# Builds both candidates' corpus predictions in ONE shared extract pass over the m2v-244
# sample columns, then runs corpus_honest_gate on each vs the STANDING CURRENT-DEFAULT reference
# (output/corpus-honest-gate/standing/baseline_m2v8m-s43.parquet — the shipped default's
# predictions + the column-intrinsic gated-YDF oracle; NOT the retired v19_gated, which scored
# deviation-from-v19, choice 0104). Build the reference once with scripts/build_standing_baseline.sh.
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 PYTHONUNBUFFERED=1

PY="eval/gittables/.venv/bin/python"
SRC="output/corpus-honest-gate/m2v-244/candidate/corpus_pass/columns.parquet"
BASELINE="output/corpus-honest-gate/standing/baseline_m2v8m-s43.parquet"
OUT="output/corpus-honest-gate/offline"
REMAP="representation.discrete.categorical=representation.text.word"
mkdir -p "$OUT"
LOG="$OUT/run.log"
exec > >(tee -a "$LOG") 2>&1

echo "================ OFFLINE GATE: potion-8M + code-16M — $(date) ================"
[[ -x ./target/release/predict_multibranch ]] || { echo "FAIL: predict_multibranch missing"; exit 1; }
[[ -f "$BASELINE" ]] || { echo "FAIL: standing baseline $BASELINE missing — build it once with scripts/build_standing_baseline.sh"; exit 1; }

echo "── Build both candidates' predictions (shared extract pass) — $(date) ──"
"$PY" scripts/offline_corpus_gate.py --source "$SRC" \
  --potion minishlab/potion-base-8M:m2v8m \
  --potion minishlab/potion-code-16M:m2v-code16m \
  --out-dir "$OUT" --jobs 8 --batch-tables 500

for tag in m2v8m m2v-code16m; do
  echo ""; echo "── Corpus-honest gate: $tag — $(date) ──"
  "$PY" scripts/corpus_honest_gate.py \
    --baseline "$BASELINE" \
    --candidate "$OUT/${tag}_candidate.parquet" \
    --label-remap "$REMAP" --label "$tag" --out-dir "$OUT/$tag" \
    | tee "$OUT/${tag}_gate.txt"
done

echo ""; echo "── Verdict summary ──"
for tag in m2v8m m2v-code16m; do
  v=$("$PY" -c "import json;print(json.load(open('$OUT/$tag/'+[f for f in __import__('os').listdir('$OUT/$tag') if f.endswith('.json')][0]))['verdict'])" 2>/dev/null || grep -oE '"verdict": "[A-Z-]+"' "$OUT/${tag}_gate.txt" | head -1)
  echo "  $tag: $v"
done
echo "================ DONE — $(date) ================"
echo "diff triggers: $OUT/m2v8m_gate.txt vs $OUT/m2v-code16m_gate.txt vs m2v-244"
