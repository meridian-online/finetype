#!/usr/bin/env bash
# attneg2 post-train verdict chain: full-model damage recovery -> post-train
# sense snapshot + drift -> candidate 33k gate pass. One-shot, evidence for the
# pre-registered swap bar (retrain_recipe_draft.md).
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 PYTHONUNBUFFERED=1
PY="eval/gittables/.venv/bin/python"
M="models/attneg2-s44"

echo "== [1/3] full-model damage recovery =="
"$PY" scripts/damage_recovery_check.py --model "$M" \
  --out output/attneg-retrain/damage_recovery_attneg2_full.json

echo "== [2/3] post-train sense snapshot + drift =="
FINETYPE_BIN="$PWD/target/release/finetype" FINETYPE_MODEL="$M" \
  "$PY" scripts/snapshot_sense_distribution.py --label attneg2_post \
  --file-list output/destination-drift-precheck/sense_dist_v19fx_s42.files.txt \
  --out-dir output/destination-drift-precheck
"$PY" scripts/drift_report.py \
  output/destination-drift-precheck/sense_dist_m2v8mfx_s43.json \
  output/destination-drift-precheck/sense_dist_attneg2_post.json \
  --json output/destination-drift-precheck/drift_attneg2_post.json || true

echo "== [3/3] candidate 33k gate pass =="
rm -rf output/attneg-retrain/cand2_pass
FINETYPE_MODEL="$M" "$PY" scripts/gittables_corpus_pass.py \
  --corpus-index output/corpus-honest-gate/stratified_sample.files.txt \
  --finetype-bin ./target/release/finetype --execute --jobs 8 \
  --out-dir output/attneg-retrain/cand2_pass
echo "VERDICT_CHAIN_DONE"
