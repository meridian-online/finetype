#!/usr/bin/env bash
# Blast radius of the `datetime.date.compact_dmy` tightening, measured as a REAL
# two-sided profile pass over the corpus tables that hold compact-date columns.
#
# Why a fresh pass on both sides and not a regex simulation. The per-label
# validator pass-rate vector is an INPUT to the multi-branch model's validation
# branch, so editing a validator moves a model input and the Sense stage itself
# differs between the two sides. Nothing short of a genuine profile pass per side
# sees that. (Same reason `compact_ymd_gate.sh` exists and the fast gate cannot
# substitute for it.)
#
# Why TWO BINARIES and not one binary with the YAML swapped. Half of this change
# is `labels/definitions_datetime.yaml`, which `load_taxonomy` reads from disk at
# runtime — swappable. The other half is `labels/veto_safe.txt`, which
# `finetype-core` pulls in with `include_str!` at COMPILE time. Swapping only the
# YAML would run the baseline pass with the candidate's hard-veto scope already
# in force and understate the change. So each side gets its own build.
#
# Why a TARGETED file list and not the 33,250-file stratified sample. The
# question is "how many columns type compact_dmy, and how many does the change
# move" — every such column lives in a table that already showed a compact-date
# column in the stratified pass, so restricting to those tables is lossless for
# the numerator and costs ~90s a side instead of ~29min. The denominator is
# reported as the columns profiled in those tables, NOT as the whole corpus.
#
# The file list is derived here, not hard-coded, from a prior stratified pass.
#
# ALWAYS restores the working-tree label files and the candidate binary on exit.
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
SAFE=labels/veto_safe.txt
PY=./eval/gittables/.venv/bin/python
BIN=./target/release/finetype

[ -f "$PRIOR" ] || { echo "FAIL: no prior corpus pass at $PRIOR"; exit 1; }
mkdir -p "$WD"

cp "$YAML" "$WD/candidate_yaml_backup"
cp "$SAFE" "$WD/candidate_safe_backup"
restore() {
  echo "== [trap] restoring the candidate label files on disk =="
  # `git checkout <ref> -- <path>` writes the INDEX as well as the worktree, so
  # a plain `cp` back would leave the files staged-modified against HEAD.
  git restore --staged "$YAML" "$SAFE" 2>/dev/null || true
  cp "$WD/candidate_yaml_backup" "$YAML"
  cp "$WD/candidate_safe_backup" "$SAFE"
}
trap restore EXIT

echo "== [1/6] derive the file list: every table holding a compact-date column =="
duckdb -noheader -list -c "
  SELECT DISTINCT file_path
  FROM read_parquet('$PRIOR')
  WHERE sense_prediction LIKE 'datetime.date.compact%'
  ORDER BY 1" > "$WD/files.txt"
echo "   $(wc -l < "$WD/files.txt") tables"

echo "== [2/6] build the CANDIDATE binary (working tree) =="
cargo build --release -p finetype-cli
cp "$BIN" "$WD/finetype-candidate"

echo "== [3/6] build the BASELINE binary ($BASE_REF label files) =="
git checkout "$BASE_REF" -- "$YAML" "$SAFE"
grep -q "datetime.date.compact_dmy" "$YAML"
cargo build --release -p finetype-cli
cp "$BIN" "$WD/finetype-baseline"
restore

echo "== [4/6] candidate pass =="
rm -rf "$WD/cand_pass"
$PY scripts/gittables_corpus_pass.py --corpus-index "$WD/files.txt" \
    --finetype-bin "$WD/finetype-candidate" --execute --jobs 8 --out-dir "$WD/cand_pass"

echo "== [5/6] baseline pass =="
rm -rf "$WD/base_pass"
$PY scripts/gittables_corpus_pass.py --corpus-index "$WD/files.txt" \
    --finetype-bin "$WD/finetype-baseline" --execute --jobs 8 --out-dir "$WD/base_pass"

echo "== [6/6] label counts and the compact_dmy transition matrix =="
duckdb -c "
  CREATE OR REPLACE VIEW b AS SELECT * FROM read_parquet('$WD/base_pass/corpus_pass/columns.parquet');
  CREATE OR REPLACE VIEW c AS SELECT * FROM read_parquet('$WD/cand_pass/corpus_pass/columns.parquet');

  SELECT '== columns profiled ==' AS section;
  SELECT (SELECT count(*) FROM b) AS baseline_columns, (SELECT count(*) FROM c) AS candidate_columns;

  SELECT '== compact-date family, before -> after ==' AS section;
  SELECT label, coalesce(n_base, 0) AS n_base, coalesce(n_cand, 0) AS n_cand
  FROM (SELECT sense_prediction AS label, count(*) AS n_base FROM b
        WHERE sense_prediction LIKE 'datetime.date.compact%' GROUP BY 1)
  FULL OUTER JOIN (SELECT sense_prediction AS label, count(*) AS n_cand FROM c
        WHERE sense_prediction LIKE 'datetime.date.compact%' GROUP BY 1) USING (label)
  ORDER BY label;

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
