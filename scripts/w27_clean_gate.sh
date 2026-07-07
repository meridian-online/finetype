#!/usr/bin/env bash
# Clean W2.7 rule-change gate: build a genuine pre-W2.7 binary (shape-only
# compact-date validators), run a fresh pre-W2.7 pass on the SAME 33k sample as
# trim_m2v8m-s43 (post-W2.7), join the oracle. The corpus-honest gate then
# isolates W2.7's effect on the shipped default with no stale-baseline confound.
# ALWAYS restores the W2.7 tree + rebuilds the W2.7 release binary on exit.
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 PYTHONUNBUFFERED=1
PY="eval/gittables/.venv/bin/python"
YAML="labels/definitions_datetime.yaml"

restore() {
  echo "== [trap] restoring W2.7 tree + binary =="
  git checkout HEAD -- "$YAML" || true
  cargo build --release -q || true
}
trap restore EXIT

echo "== [1/4] revert compact-date validators to pre-W2.7 (shape-only) + build =="
git checkout 879931a^ -- "$YAML"
grep -A1 "^datetime.date.compact_ymd:" "$YAML" >/dev/null  # sanity file exists
cargo build --release -q
./target/release/finetype --version

echo "== [2/4] pre-W2.7 default pass on the stratified 33k sample =="
rm -rf output/attneg-retrain/preW27_m2v8m-s43_pass
FINETYPE_MODEL=models/m2v8m-s43 "$PY" scripts/gittables_corpus_pass.py \
  --corpus-index output/corpus-honest-gate/stratified_sample.files.txt \
  --finetype-bin ./target/release/finetype --execute --jobs 8 \
  --out-dir output/attneg-retrain/preW27_m2v8m-s43_pass

echo "== [3/4] restore W2.7 tree + binary (explicit; trap is the backstop) =="
git checkout HEAD -- "$YAML"
cargo build --release -q
./target/release/finetype --version

echo "== [4/4] join oracle onto the pre-W2.7 pass =="
duckdb -c "COPY (SELECT b.file_path, b.column_name, b.sense_prediction, o.ydf_prediction_gated
  FROM read_parquet('output/attneg-retrain/preW27_m2v8m-s43_pass/corpus_pass/columns.parquet') b
  LEFT JOIN read_parquet('output/ydf-validation-gate/v19_gated.parquet') o USING (file_path, column_name))
  TO 'output/attneg-retrain/preW27_baseline_with_oracle.parquet' (FORMAT parquet)"

echo "W27_CLEAN_GATE_PREP_DONE"
