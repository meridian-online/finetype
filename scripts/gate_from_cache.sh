#!/usr/bin/env bash
# Gate a Sharpen-rule change reusing an EXISTING raw-Sense cache (skips the ~6min encode
# pass that corpus_honest_gate_fast.sh does internally). The cache is model-intrinsic —
# rules only change the resharpen step — so one cache gates every rule on the same model.
#
# Usage:
#   gate_from_cache.sh <cache.tsv> <candidate-bin> <baseline-bin> <workdir> [label]
#     candidate-bin : binary with the rule ON
#     baseline-bin  : same model with the rule OFF
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
PY=./eval/gittables/.venv/bin/python
CACHE="$1"; CUR="$2"; BASE="$3"; WD="$4"; LABEL="${5:-rule-candidate}"
ORACLE="output/ydf-validation-gate/v19_gated.parquet"
[ -f "$CACHE" ] || { echo "FAIL: cache $CACHE"; exit 1; }
[ -x "$CUR" ] || { echo "FAIL: candidate binary $CUR"; exit 1; }
[ -x "$BASE" ] || { echo "FAIL: baseline binary $BASE"; exit 1; }
mkdir -p "$WD"

echo "── resharpen: candidate (rule ON) + baseline (rule OFF) ──"
$CUR  resharpen -i "$CACHE" -o "$WD/cand.tsv"
$BASE resharpen -i "$CACHE" -o "$WD/base.tsv"

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
