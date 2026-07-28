#!/usr/bin/env bash
# Blast radius of the `datetime.date.compact_dmy` validator tightening,
# measured as a REAL two-sided profile pass over the corpus tables that hold
# compact-date columns.
#
# Why a fresh pass on both sides and not a regex simulation. The validator's
# per-label pass-rate vector is an INPUT to the multi-branch model's validation
# branch, so editing a validator moves a model input and the Sense stage itself
# differs between the two sides. Nothing short of a genuine profile pass per
# side sees that. (Same reason `compact_ymd_gate.sh` exists and the fast gate
# cannot substitute for it.)
#
# Why a TARGETED file list and not the 33,250-file stratified sample. The
# question this script answers is "how many columns currently type
# compact_dmy, and how many does the change move" — every such column lives in
# a table that already showed a compact-date column in the stratified pass, so
# restricting to those tables is lossless for the numerator and costs ~90s a
# side instead of ~29min. The denominator is reported as the columns profiled
# in those tables, not as the whole corpus, and the report says so.
#
# The file list is derived here, not hard-coded, from a prior stratified pass.
#
# ALWAYS restores the working-tree YAML on exit.
#
# Usage:
#   compact_dmy_blast_radius.sh <workdir> [base-ref] [prior-pass-parquet]
#     base-ref : git ref holding the PRE-change validator (default origin/main)
set -eo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 PYTHONUNBUFFERED=1

WD="${1:?usage: compact_dmy_blast_radius.sh <workdir> [base-ref] [prior-pass-parquet]}"
BASE_REF="${2:-origin/main}"
PRIOR="${3:-output/compact-ymd-gate/full/cand_pass/corpus_pass/columns.parquet}"
YAML=labels/definitions_datetime.yaml
PY=./eval/gittables/.venv/bin/python
BIN=./target/release/finetype

[ -x "$BIN" ] || { echo "FAIL: no release binary at $BIN"; exit 1; }
[ -f "$PRIOR" ] || { echo "FAIL: no prior corpus pass at $PRIOR"; exit 1; }
mkdir -p "$WD"

CAND_YAML="$WD/candidate_yaml_backup"
cp "$YAML" "$CAND_YAML"
restore() {
  echo "== [trap] restoring the candidate validator on disk =="
  git restore --staged "$YAML" 2>/dev/null || true
  cp "$CAND_YAML" "$YAML"
}
trap restore EXIT

echo "== [1/5] derive the file list: every table holding a compact-date column =="
duckdb -noheader -list -c "
  SELECT DISTINCT file_path
  FROM read_parquet('$PRIOR')
  WHERE sense_prediction LIKE 'datetime.date.compact%'
  ORDER BY 1" > "$WD/files.txt"
echo "   $(wc -l < "$WD/files.txt") tables"

echo "== [2/5] candidate pass (day-first validator TIGHTENED) =="
rm -rf "$WD/cand_pass"
$PY scripts/gittables_corpus_pass.py --corpus-index "$WD/files.txt" \
    --finetype-bin "$BIN" --execute --jobs 8 --out-dir "$WD/cand_pass"

echo "== [3/5] revert the validator to $BASE_REF, baseline pass (shape-only) =="
git checkout "$BASE_REF" -- "$YAML"
grep -q "datetime.date.compact_dmy" "$YAML"
rm -rf "$WD/base_pass"
$PY scripts/gittables_corpus_pass.py --corpus-index "$WD/files.txt" \
    --finetype-bin "$BIN" --execute --jobs 8 --out-dir "$WD/base_pass"

echo "== [4/5] restore the candidate validator (explicit; trap is the backstop) =="
git restore --staged "$YAML" 2>/dev/null || true
cp "$CAND_YAML" "$YAML"

echo "== [5/5] label counts and the compact_dmy transition matrix =="
duckdb -c "
  CREATE OR REPLACE VIEW b AS SELECT * FROM read_parquet('$WD/base_pass/corpus_pass/columns.parquet');
  CREATE OR REPLACE VIEW c AS SELECT * FROM read_parquet('$WD/cand_pass/corpus_pass/columns.parquet');

  SELECT '== columns profiled ==' AS section;
  SELECT (SELECT count(*) FROM b) AS baseline_columns, (SELECT count(*) FROM c) AS candidate_columns;

  SELECT '== compact-date family, before -> after ==' AS section;
  SELECT coalesce(b.sense_prediction, c.sense_prediction) AS label,
         count(b.column_name) FILTER (WHERE b.sense_prediction IS NOT NULL) AS n_base,
         count(c.column_name) FILTER (WHERE c.sense_prediction IS NOT NULL) AS n_cand
  FROM (SELECT * FROM b WHERE sense_prediction LIKE 'datetime.date.compact%') b
  FULL OUTER JOIN (SELECT * FROM c WHERE sense_prediction LIKE 'datetime.date.compact%') c
    ON b.file_path = c.file_path AND b.column_name = c.column_name
  GROUP BY 1 ORDER BY 1;

  SELECT '== where the baseline compact_dmy columns went ==' AS section;
  SELECT coalesce(c.sense_prediction, '(dropped from pass)') AS became, count(*) AS n
  FROM b LEFT JOIN c USING (file_path, column_name)
  WHERE b.sense_prediction = 'datetime.date.compact_dmy'
  GROUP BY 1 ORDER BY 2 DESC;

  SELECT '== what became compact_dmy that was not ==' AS section;
  SELECT coalesce(b.sense_prediction, '(new)') AS was, count(*) AS n
  FROM c LEFT JOIN b USING (file_path, column_name)
  WHERE c.sense_prediction = 'datetime.date.compact_dmy'
    AND coalesce(b.sense_prediction, '') <> 'datetime.date.compact_dmy'
  GROUP BY 1 ORDER BY 2 DESC;

  SELECT '== every label that moved by more than 5 columns ==' AS section;
  SELECT label, n_base, n_cand, n_cand - n_base AS delta FROM (
    SELECT coalesce(b.label, c.label) AS label, coalesce(b.n, 0) AS n_base, coalesce(c.n, 0) AS n_cand
    FROM (SELECT sense_prediction AS label, count(*) AS n FROM b GROUP BY 1) b
    FULL OUTER JOIN (SELECT sense_prediction AS label, count(*) AS n FROM c GROUP BY 1) c USING (label)
  ) WHERE abs(n_cand - n_base) > 5 ORDER BY abs(n_cand - n_base) DESC;
" | tee "$WD/blast_radius.txt"

echo "wrote $WD/blast_radius.txt"
