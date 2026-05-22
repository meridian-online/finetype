-- ac-07 — per-file two-criterion gate computed from files.parquet
--
-- Criterion (a): non_trivial_pct = non_trivial_cols / n_cols >= 0.80
-- Criterion (b): reject_rate_non_trivial = rejects_non_trivial / total_rows <= 0.01
--
-- Each measure-half file lands in exactly one of four buckets:
--   files_passed                  : passes_a AND passes_b
--   criterion_a_only_failures     : NOT passes_a AND passes_b
--   criterion_b_only_failures     : passes_a AND NOT passes_b
--   criterion_both_failures       : NOT passes_a AND NOT passes_b
--
-- gate_score = files_passed / files_total.
--
-- Files that errored during the corpus pass (error IS NOT NULL) fail
-- both criteria by construction — they can't be measured at all and
-- land in `criterion_both_failures`. Files with n_cols = 0 or
-- total_rows = 0 are likewise treated as failing the relevant
-- criterion (division by zero is replaced with `false`).
--
-- Output: eval/gittables/corpus_pass/gate_summary.json
-- Regenerable; reads files.parquet only.

COPY (
    WITH classified AS (
        SELECT
            (error IS NULL
                AND n_cols > 0
                AND CAST(non_trivial_cols AS DOUBLE) / n_cols >= 0.80) AS passes_a,
            (error IS NULL
                AND total_rows > 0
                AND CAST(rejects_non_trivial AS DOUBLE) / total_rows <= 0.01) AS passes_b
        FROM read_parquet(
            'eval/gittables/corpus_pass/files.parquet'
        )
    )
    SELECT
        COUNT(*) AS files_total,
        COUNT_IF(passes_a AND passes_b) AS files_passed,
        CAST(COUNT_IF(passes_a AND passes_b) AS DOUBLE)
            / NULLIF(COUNT(*), 0) AS gate_score,
        COUNT_IF(NOT passes_a AND passes_b) AS criterion_a_only_failures,
        COUNT_IF(passes_a AND NOT passes_b) AS criterion_b_only_failures,
        COUNT_IF(NOT passes_a AND NOT passes_b) AS criterion_both_failures,
        0.80 AS non_trivial_floor,
        0.01 AS reject_rate_ceil
    FROM classified
) TO 'eval/gittables/corpus_pass/gate_summary.json' (FORMAT 'json', ARRAY false);
