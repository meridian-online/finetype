#!/usr/bin/env bash
# Fast, constructive corpus-honest gate for a SHARPEN-RULE change (spec
# 2026-06-27-composed-accuracy-roadmap; memory corpus-honest-gate-rebaselined-to-m2v8m).
#
# Re-anchored to the reproducible shipped model (m2v8m-s43), NOT the retired v19: the
# baseline is the same model with the candidate RULE OFF, so every A->B transition reflects
# exactly the rule under test (gating vs v19 only measures the structurally-unpassable model
# drift). The gated-YDF oracle (column-intrinsic) is reused from v19_gated.parquet.
#
# Uses the re-sharpen fast path (compose_from_sense): ONE sibling-aware raw-Sense pass over
# the 33k-file stratified sample (~6min, reconstructed from cached columns.parquet values),
# then two instant resharpens (rules-off baseline + rules-on candidate). The raw-Sense cache
# is reusable — re-gating another rule on the same model is seconds.
#
# Usage:
#   corpus_honest_gate_fast.sh <workdir> <candidate-bin> <baseline-bin> [label]
#     candidate-bin : current binary (rule ON)
#     baseline-bin  : same model built with the candidate rule REVERTED (rule OFF)
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
PY=./eval/gittables/.venv/bin/python
WD="$1"; CUR="${2:-./target/release/finetype}"; BASE="$3"; LABEL="${4:-rule-candidate}"
ORACLE="output/ydf-validation-gate/v19_gated.parquet"
[ -x "$CUR" ] || { echo "FAIL: candidate binary $CUR"; exit 1; }
[ -x "$BASE" ] || { echo "FAIL: baseline binary $BASE (build the model with the rule reverted)"; exit 1; }
mkdir -p "$WD"

echo "── raw-Sense cache (one sibling-aware encode pass over the stratified sample) ──"
$PY scripts/gate_candidate_from_cache.py --bin "$CUR" --workdir "$WD/cache" --out /dev/null \
    --raw-sense-tsv "$WD/sense_cache.tsv"

echo "── resharpen: candidate (rule ON) + baseline (rule OFF) ──"
$CUR  resharpen -i "$WD/sense_cache.tsv" -o "$WD/cand.tsv"
$BASE resharpen -i "$WD/sense_cache.tsv" -o "$WD/base.tsv"

echo "── format parquets + join the gated-YDF oracle into the baseline ──"
$PY - "$WD" "$ORACLE" <<'PY'
import sys, pyarrow as pa, pyarrow.parquet as pq, duckdb
WD, ORACLE = sys.argv[1], sys.argv[2]; US = "\x1f"
def load(p):
    fps, cns, labs = [], [], []
    for line in open(p):
        x = line.rstrip("\n").split("\t")
        if len(x) < 2 or US not in x[0]:
            continue
        fp, cn = x[0].split(US, 1)
        fps.append(fp); cns.append(cn); labs.append(x[1])
    return fps, cns, labs
for tag, src in [("candidate", "cand"), ("baseline_sense", "base")]:
    fps, cns, labs = load(f"{WD}/{src}.tsv")
    pq.write_table(pa.table({"file_path": fps, "column_name": cns, "sense_prediction": labs}),
                   f"{WD}/{tag}.parquet")
duckdb.sql(f'''COPY (SELECT b.file_path, b.column_name, b.sense_prediction, o.ydf_prediction_gated
  FROM read_parquet("{WD}/baseline_sense.parquet") b
  LEFT JOIN read_parquet("{ORACLE}") o USING (file_path, column_name)
) TO "{WD}/baseline.parquet" (FORMAT parquet)''')
PY

echo "── corpus-honest gate (m2v8m rule-OFF -> rule-ON) ──"
$PY scripts/corpus_honest_gate.py --baseline "$WD/baseline.parquet" --candidate "$WD/candidate.parquet" \
    --label "$LABEL" --out-dir "$WD/gate" | tee "$WD/verdict.txt"
