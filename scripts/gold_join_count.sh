#!/usr/bin/env bash
# Matched-row count for the evaluation substrate's join.
#
# WHY THIS EXISTS. The gittables corpus path in eval/gold/gold_corpus.tsv is a
# JOIN KEY: columns.parquet and v19_gated.parquet match on it. Rewriting it on
# one side and not the others does not error — the join simply matches fewer
# rows, and every gate downstream returns a credible-looking figure. So any sweep
# that could touch those strings has to show a matched-row count before and
# after, not a diff that looks harmless.
#
# The two parquets are regenerable and gitignored (~300MB each), so this cannot
# run in CI. It runs where they live.
#
# Usage:
#   scripts/gold_join_count.sh [gold_corpus.tsv]
#
#   # before/after around a sweep:
#   git show origin/main:eval/gold/gold_corpus.tsv > /tmp/gold_before.tsv
#   scripts/gold_join_count.sh /tmp/gold_before.tsv
#   scripts/gold_join_count.sh eval/gold/gold_corpus.tsv
#
# Every number must be identical on both sides. If `gold_gittables_paths` moves
# without `join_columns_pathcol` moving with it, the key was rewritten on one
# side only — which is the failure this script is for.
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
GOLD="${1:-$ROOT/eval/gold/gold_corpus.tsv}"
COLS="${FINETYPE_COLUMNS_PARQUET:-$ROOT/eval/gittables/corpus_pass/columns.parquet}"
V19="${FINETYPE_V19_PARQUET:-$ROOT/output/ydf-validation-gate/v19_gated.parquet}"

for f in "$GOLD" "$COLS" "$V19"; do
	if [[ ! -f "$f" ]]; then
		echo "gold_join_count: missing $f" >&2
		echo "    the parquets are regenerable and gitignored; run this where they live," >&2
		echo "    or point FINETYPE_COLUMNS_PARQUET / FINETYPE_V19_PARQUET at them." >&2
		exit 2
	fi
done

duckdb -no-init -noheader -list -c "
CREATE OR REPLACE TEMP VIEW gold AS
  SELECT * FROM read_csv('$GOLD', delim='\t', header=true, all_varchar=true);

SELECT 'gold_rows',              (SELECT count(*) FROM gold);
SELECT 'gold_gittables_paths',   (SELECT count(*) FROM gold WHERE file_path LIKE '%/datasets/gittables/%');
SELECT 'join_columns_pathcol',   (SELECT count(*) FROM gold g JOIN read_parquet('$COLS') c
                                    ON g.file_path = c.file_path AND g.column_name = c.column_name);
SELECT 'join_v19_pathcol',       (SELECT count(*) FROM gold g JOIN read_parquet('$V19') v
                                    ON g.file_path = v.file_path AND g.column_name = v.column_name);
SELECT 'join_columns_shacol',    (SELECT count(*) FROM gold g JOIN read_parquet('$COLS') c
                                    ON g.file_content_sha256 = c.file_content_sha256 AND g.column_name = c.column_name);
SELECT 'join_v19_shacol',        (SELECT count(*) FROM gold g JOIN read_parquet('$V19') v
                                    ON g.file_content_sha256 = v.file_content_sha256 AND g.column_name = v.column_name);
SELECT 'distinct_paths_matched', (SELECT count(DISTINCT g.file_path) FROM gold g
                                    JOIN read_parquet('$COLS') c ON g.file_path = c.file_path);
"
