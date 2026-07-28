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
# EACH SIDE IS BUILT **AND RUN** INSIDE ITS OWN LABEL STATE. The change spans
# two files that reach the binary by different routes:
#
#   labels/veto_safe.txt              `include_str!` at COMPILE time
#       (crates/finetype-core/src/validation_veto.rs), so each side needs its
#       own build.
#   labels/definitions_datetime.yaml  read from `./labels` AT RUNTIME —
#       `profile` hard-codes `PathBuf::from("labels")` against the process's
#       working directory, and `gittables_corpus_pass.py` spawns it with no
#       `cwd=`, so the pass inherits this script's directory: the repo root.
#
# An earlier revision built both binaries, restored the candidate label files,
# and only then ran both passes — which ran the BASELINE binary against the
# CANDIDATE taxonomy, so the "before" side already had the tightening in force
# and the measured blast radius was near zero by construction. Ordering the two
# phases per side is the whole fix, and the on-disk blob shas are recorded so
# the report names exactly what produced each side.
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
PRIOR_BASE="${4:-output/compact-ymd-gate/full/base_pass/corpus_pass/columns.parquet}"
YAML=labels/definitions_datetime.yaml
SAFE=labels/veto_safe.txt
PY=./eval/gittables/.venv/bin/python
BIN=./target/release/finetype

[ -f "$PRIOR" ] || { echo "FAIL: no prior corpus pass at $PRIOR"; exit 1; }
[ -f "$PRIOR_BASE" ] || { echo "FAIL: no prior corpus pass at $PRIOR_BASE"; exit 1; }
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

echo "== [1/4] derive the file list: every table holding a compact-date column =="
# UNION of both sides of the prior gate, not just its candidate side: a table
# whose only compact-date column was pushed OFF the family by that change would
# be missing from the candidate side alone, and those are exactly the tables
# this change might move back.
duckdb -noheader -list -c "
  SELECT DISTINCT file_path FROM read_parquet('$PRIOR')
  WHERE sense_prediction LIKE 'datetime.date.compact%'
  UNION
  SELECT DISTINCT file_path FROM read_parquet('$PRIOR_BASE')
  WHERE sense_prediction LIKE 'datetime.date.compact%'
  ORDER BY 1" > "$WD/files.txt"
echo "   $(wc -l < "$WD/files.txt") tables"

# NOT truncated here: under REPORT_ONLY the provenance of the passes already on
# disk is the only record of what produced them, and clearing it would leave the
# report unable to name its own inputs. `run_pass` truncates on its first call.
run_pass() {  # run_pass <tag> <ref|"">
  local tag="$1" ref="$2"
  [ "$tag" = base ] && : > "$WD/side_provenance.tsv"
  echo "== side $tag (${ref:-working tree}) : place label files =="
  if [ -n "$ref" ]; then git checkout "$ref" -- "$YAML" "$SAFE"; else restore; fi
  grep -q "datetime.date.compact_dmy" "$YAML"
  printf '%s\t%s\t%s\t%s\n' "$tag" "${ref:-working-tree}" \
    "$(git hash-object "$YAML")" "$(git hash-object "$SAFE")" >> "$WD/side_provenance.tsv"
  echo "== side $tag : build (veto_safe.txt is compiled in) =="
  cargo build --release -p finetype-cli
  echo "== side $tag : pass (definitions_datetime.yaml is read from ./labels) =="
  rm -rf "$WD/${tag}_pass"
  $PY scripts/gittables_corpus_pass.py --corpus-index "$WD/files.txt" \
      --finetype-bin "$BIN" --execute --jobs 8 --out-dir "$WD/${tag}_pass"
}

# REPORT_ONLY=1 re-derives the report from passes already on disk. The two
# corpus passes cost ~20 min; the analysis below is seconds and gets edited far
# more often than the passes get re-run.
if [ "${REPORT_ONLY:-0}" = "1" ]; then
  echo "== REPORT_ONLY: reusing the passes already in $WD =="
  [ -f "$WD/base_pass/corpus_pass/columns.parquet" ] || { echo "FAIL: no base pass in $WD"; exit 1; }
  [ -f "$WD/cand_pass/corpus_pass/columns.parquet" ] || { echo "FAIL: no cand pass in $WD"; exit 1; }
  [ -s "$WD/side_provenance.tsv" ] || { echo "FAIL: no side provenance in $WD"; exit 1; }
else
  echo "== [2/4] BASELINE side ($BASE_REF) =="
  run_pass base "$BASE_REF"

  echo "== [3/4] CANDIDATE side (working tree) =="
  run_pass cand ""
fi

echo "== [4/4] label counts and the compact_dmy transition matrix =="
{
  echo "# compact_dmy blast radius"
  echo "# head_sha  $(git rev-parse HEAD)"
  echo "# base_ref  $BASE_REF  $(git rev-parse "$BASE_REF")"
  echo "# taxonomy_version  $(python3 scripts/evidence.py taxonomy-version 2>/dev/null | tr -d '\n' || echo unknown)"
  echo "# tables    $(wc -l < "$WD/files.txt" | tr -d ' ')"
  echo "# side  ref  definitions_datetime.yaml_blob  veto_safe.txt_blob"
  sed 's/^/# /' "$WD/side_provenance.tsv"
} > "$WD/blast_radius.txt"
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

  -- ═══ THE COLLATERAL, WHICH IS THE PART THAT COSTS SOMETHING ═══════════════
  -- A validator's per-label pass rate is an INPUT to the multi-branch model's
  -- validation branch. Tightening the DAY-FIRST leaf therefore moves a feature
  -- on every eight-digit column in the corpus, including YEAR-FIRST ones: a
  -- genuine YYYYMMDD value scores 1.0 on the day-first validator before this
  -- change and 0.0 after it, because its middle pair is a day-of-month and
  -- overflows the month window. The sections below measure what that costs.
  SELECT '== columns that LOST datetime.date.compact_ymd ==' AS section;
  SELECT coalesce(c.sense_prediction, '(dropped from pass)') AS became, count(*) AS n
  FROM b JOIN c USING (file_path, column_name)
  WHERE b.sense_prediction = 'datetime.date.compact_ymd'
    AND c.sense_prediction IS DISTINCT FROM 'datetime.date.compact_ymd'
  GROUP BY 1 ORDER BY 2 DESC;

  SELECT '== ...and were they GENUINE dates? (every sampled value a valid YYYYMMDD) ==' AS section;
  SELECT count(*) AS n_moved,
         count(*) FILTER (WHERE all_ymd) AS all_values_valid_yyyymmdd,
         count(*) FILTER (WHERE NOT all_ymd) AS not_all_valid
  FROM (
    SELECT list_reduce(list_transform(str_split(b.sample_values_truncated, '│'),
             x -> CASE WHEN regexp_matches(trim(x),
                    '^\d{4}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\$') THEN 1 ELSE 0 END),
             (a, x) -> a * x) = 1 AS all_ymd
    FROM b JOIN c USING (file_path, column_name)
    WHERE b.sense_prediction = 'datetime.date.compact_ymd'
      AND c.sense_prediction IS DISTINCT FROM 'datetime.date.compact_ymd'
  );

  SELECT '== ...under which headers (names only, never corpus values) ==' AS section;
  SELECT lower(b.column_name) AS header, count(*) AS n
  FROM b JOIN c USING (file_path, column_name)
  WHERE b.sense_prediction = 'datetime.date.compact_ymd'
    AND c.sense_prediction IS DISTINCT FROM 'datetime.date.compact_ymd'
  GROUP BY 1 ORDER BY 2 DESC;

  SELECT '== same question for the compact_mdy losses ==' AS section;
  SELECT coalesce(c.sense_prediction, '(dropped from pass)') AS became, count(*) AS n
  FROM b JOIN c USING (file_path, column_name)
  WHERE b.sense_prediction = 'datetime.date.compact_mdy'
    AND c.sense_prediction IS DISTINCT FROM 'datetime.date.compact_mdy'
  GROUP BY 1 ORDER BY 2 DESC;

  -- The trade, stated as one row so it cannot be quoted selectively.
  SELECT '== THE TRADE ==' AS section;
  SELECT
    (SELECT count(*) FROM b WHERE sense_prediction = 'datetime.date.compact_dmy')
      AS day_first_before,
    (SELECT count(*) FROM c WHERE sense_prediction = 'datetime.date.compact_dmy')
      AS day_first_after,
    (SELECT count(*) FROM b JOIN c USING (file_path, column_name)
      WHERE b.sense_prediction = 'datetime.date.compact_ymd'
        AND c.sense_prediction IS DISTINCT FROM 'datetime.date.compact_ymd')
      AS genuine_ymd_columns_lost,
    (SELECT count(*) FROM c JOIN b USING (file_path, column_name)
      WHERE c.sense_prediction = 'datetime.date.compact_dmy'
        AND b.sense_prediction IS DISTINCT FROM 'datetime.date.compact_dmy')
      AS newly_typed_day_first;
" | tee -a "$WD/blast_radius.txt"

# `output/` is blanket-gitignored as derived experiment scratch, and a number
# nobody else can open is not evidence. The report is copied into `docs/`.
cp "$WD/blast_radius.txt" docs/compact-dmy-blast-radius.txt
echo "wrote docs/compact-dmy-blast-radius.txt"

echo "wrote $WD/blast_radius.txt"
