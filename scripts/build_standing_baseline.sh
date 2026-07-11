#!/usr/bin/env bash
# Build the STANDING CURRENT-DEFAULT reference pass for the corpus-honest gate.
#
# Since de-fossilisation (corpus_honest_gate.py: --baseline is REQUIRED, no v19 default),
# every gate runbook needs a baseline = the SHIPPED DEFAULT model's predictions on the ac-01
# stratified sample, carrying sense_prediction + the column-intrinsic gated-YDF oracle. This
# builds that ONE standing parquet so gate_m2v244.sh / offline_gate_run.sh / eval_rule.sh can
# all point at it instead of the retired v19 (which scored deviation-from-v19 — choice 0104).
#
# Uses the resharpen fast path (compose_from_sense): ONE sibling-aware raw-Sense encode pass
# over the 33k sample (~6 min), then resharpen with the current default -> composed labels,
# then join the gated-YDF oracle. Rebuild whenever models/default changes.
#
# Output: output/corpus-honest-gate/standing/baseline_<tag>.parquet
# Usage:  scripts/build_standing_baseline.sh [candidate-bin] [tag]
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
PY=./eval/gittables/.venv/bin/python
BIN="${1:-./target/release/finetype}"          # current default binary (all shipped rules ON)
TAG="${2:-m2v8m-s43}"                           # names the output; the shipped default
ORACLE="output/ydf-validation-gate/v19_gated.parquet"
OUT="output/corpus-honest-gate/standing"
WD="$OUT/_build"
DEST="$OUT/baseline_${TAG}.parquet"
[ -x "$BIN" ] || { echo "FAIL: candidate binary $BIN missing"; exit 1; }
[ -f "$ORACLE" ] || { echo "FAIL: gated-YDF oracle $ORACLE missing"; exit 1; }
mkdir -p "$WD"

echo "── raw-Sense cache (one sibling-aware encode pass over the stratified sample) — $(date) ──"
$PY scripts/gate_candidate_from_cache.py --bin "$BIN" --workdir "$WD/cache" --out /dev/null \
    --raw-sense-tsv "$WD/sense_cache.tsv"

echo "── resharpen with the current default (rules ON) -> composed labels ──"
$BIN resharpen -i "$WD/sense_cache.tsv" -o "$WD/pass.tsv"

echo "── format parquet + join the gated-YDF oracle ──"
$PY - "$WD" "$ORACLE" "$DEST" <<'PY'
import sys, pyarrow as pa, pyarrow.parquet as pq, duckdb
WD, ORACLE, DEST = sys.argv[1], sys.argv[2], sys.argv[3]; US = "\x1f"
fps, cns, labs = [], [], []
for line in open(f"{WD}/pass.tsv"):
    x = line.rstrip("\n").split("\t")
    if len(x) < 2 or US not in x[0]:
        continue
    fp, cn = x[0].split(US, 1)
    fps.append(fp); cns.append(cn); labs.append(x[1])
pq.write_table(pa.table({"file_path": fps, "column_name": cns, "sense_prediction": labs}),
               f"{WD}/pass.parquet")
duckdb.sql(f'''COPY (SELECT b.file_path, b.column_name, b.sense_prediction, o.ydf_prediction_gated
  FROM read_parquet("{WD}/pass.parquet") b
  LEFT JOIN read_parquet("{ORACLE}") o USING (file_path, column_name)
) TO "{DEST}" (FORMAT parquet)''')
n = pq.read_metadata(DEST).num_rows
print(f"  wrote {DEST} ({n} rows)")
PY

rm -rf "$WD"
echo "── DONE — standing baseline: $DEST — $(date) ──"
