#!/usr/bin/env bash
# Full-pass corpus-honest gate for a TAXONOMY-VALIDATOR change.
#
# Why not `corpus_honest_gate_fast.sh`. The fast path resharpens ONE cached
# raw-Sense pass twice (rule off / rule on), so it can only see a change that
# lives in the Sharpen layer. A validator edit is invisible to it twice over:
#
#   1. `resharpen` composes through `compose_from_sense` and never runs the
#      hard validation veto — that lives in the profile path — and the veto is
#      exactly what a tightened validator acts through.
#   2. The taxonomy is read from `labels/` on disk at runtime whenever that
#      directory exists (`load_taxonomy`, model_loading.rs), and the embedded
#      copy is only the fallback. So a separately-built "baseline binary" reads
#      the candidate's YAML anyway, and the two resharpens are identical.
#
# There is a third reason a validator change needs a fresh pass on both sides:
# the per-label validator pass-rate vector is an INPUT to the multi-branch
# model's validation branch (`valid_dim` in the model config). Editing a
# validator moves a model input, so the Sense stage itself differs — and the
# fast path holds Sense common-mode by construction.
#
# So: swap the YAML on disk, run a genuine profile pass for each side over the
# same stratified sample, join the stable gated-YDF oracle onto the baseline,
# and score. Same instrument the earlier compact-date attempt was measured on.
#
# ALWAYS restores the working-tree YAML on exit.
#
# Usage:
#   compact_ymd_gate.sh <workdir> [base-ref] [yaml] [label]
#     base-ref : git ref holding the PRE-change validator (default origin/main —
#                NOT HEAD~1, which drifts as soon as a second commit lands on
#                the branch and would silently gate the change against itself)
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 PYTHONUNBUFFERED=1

WD="${1:?usage: compact_ymd_gate.sh <workdir> [base-ref] [yaml] [label]}"
BASE_REF="${2:-origin/main}"
YAML="${3:-labels/definitions_datetime.yaml}"
LABEL="${4:-compact-ymd-range}"
PY=./eval/gittables/.venv/bin/python
BIN=./target/release/finetype
SAMPLE=output/corpus-honest-gate/stratified_sample.files.txt
ORACLE=output/ydf-validation-gate/v19_gated.parquet

[ -x "$BIN" ] || { echo "FAIL: no release binary at $BIN"; exit 1; }
[ -f "$SAMPLE" ] || { echo "FAIL: no stratified sample at $SAMPLE"; exit 1; }
[ -f "$ORACLE" ] || { echo "FAIL: no gated oracle at $ORACLE"; exit 1; }
mkdir -p "$WD"
CAND_YAML="$WD/candidate_yaml_backup"
cp "$YAML" "$CAND_YAML"

# `git checkout <ref> -- <path>` writes the INDEX as well as the worktree, so a
# plain `cp` back leaves the file staged-modified against HEAD (`MM`). Unstage
# first, then restore the bytes.
restore() {
  echo "== [trap] restoring the candidate validator on disk =="
  git restore --staged "$YAML" 2>/dev/null || true
  cp "$CAND_YAML" "$YAML"
}
trap restore EXIT

echo "== [1/4] candidate pass (validator ON) over the stratified sample =="
rm -rf "$WD/cand_pass"
$PY scripts/gittables_corpus_pass.py --corpus-index "$SAMPLE" \
    --finetype-bin "$BIN" --execute --jobs 8 --out-dir "$WD/cand_pass"

echo "== [2/4] revert the validator to $BASE_REF, baseline pass (validator OFF) =="
git checkout "$BASE_REF" -- "$YAML"
grep -q "datetime.date.compact_ymd" "$YAML"
rm -rf "$WD/base_pass"
$PY scripts/gittables_corpus_pass.py --corpus-index "$SAMPLE" \
    --finetype-bin "$BIN" --execute --jobs 8 --out-dir "$WD/base_pass"

echo "== [3/4] restore the candidate validator (explicit; trap is the backstop) =="
git restore --staged "$YAML" 2>/dev/null || true
cp "$CAND_YAML" "$YAML"

echo "== [4/4] join the gated oracle onto the baseline, then score =="
duckdb -c "COPY (SELECT b.file_path, b.column_name, b.sense_prediction, o.ydf_prediction_gated
  FROM read_parquet('$WD/base_pass/corpus_pass/columns.parquet') b
  LEFT JOIN read_parquet('$ORACLE') o USING (file_path, column_name))
  TO '$WD/baseline.parquet' (FORMAT parquet)"

$PY scripts/corpus_honest_gate.py \
    --baseline "$WD/baseline.parquet" \
    --candidate "$WD/cand_pass/corpus_pass/columns.parquet" \
    --label "$LABEL" --out-dir "$WD/gate" | tee "$WD/verdict.txt"
