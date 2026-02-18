-- GitTables Evaluation: FineType vs Schema.org/DBpedia semantic type annotations
-- =============================================================================
-- Evaluates FineType format detection against real-world column data from the
-- GitTables benchmark (1,101 tables, ~3,200 annotated columns).

LOAD '${EXTENSION_PATH}';

.mode box
.timer on

-- ═══════════════════════════════════════════════════════════════════════════════
-- 1. LOAD GROUND TRUTH
-- ═══════════════════════════════════════════════════════════════════════════════

CREATE OR REPLACE TABLE schema_gt AS
SELECT
    replace(table_id, '_schema', '') AS table_file,
    target_column AS col_idx,
    annotation_label AS gt_label,
    'schema.org' AS ontology
FROM read_csv('eval/gittables/schema_gt.csv', auto_detect=true);

CREATE OR REPLACE TABLE dbpedia_gt AS
SELECT
    replace(table_id, '_dbpedia', '') AS table_file,
    target_column AS col_idx,
    annotation_label AS gt_label,
    'dbpedia' AS ontology
FROM read_csv('eval/gittables/dbpedia_gt.csv', auto_detect=true);

-- Prefer schema.org annotations, fall back to dbpedia
CREATE OR REPLACE TABLE ground_truth AS
SELECT * FROM schema_gt
UNION ALL
SELECT d.* FROM dbpedia_gt d
WHERE NOT EXISTS (
    SELECT 1 FROM schema_gt s
    WHERE s.table_file = d.table_file AND s.col_idx = d.col_idx
);

SELECT 'Ground truth' AS step,
       count(*) AS total_annotations,
       count(DISTINCT table_file) AS unique_tables,
       count(DISTINCT gt_label) AS unique_labels
FROM ground_truth;

-- Label distribution in ground truth
.print ''
.print '--- Top 20 ground truth labels ---'
SELECT gt_label, count(*) AS cnt
FROM ground_truth
GROUP BY gt_label
ORDER BY cnt DESC
LIMIT 20;

-- ═══════════════════════════════════════════════════════════════════════════════
-- 2. EXTRACT COLUMN VALUES VIA UNPIVOT
-- For each table, we read the CSV, UNPIVOT all columns to rows,
-- and join with ground truth to get only annotated columns.
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '--- Extracting column values from tables ---'

-- Read ALL tables into a single table, adding table_file identifier
-- Tables have columns: column00, col0, col1, ..., col{N}
-- We'll read all_varchar so everything is strings

-- First, create a file list
CREATE OR REPLACE TABLE csv_files AS
SELECT
    replace(replace(file, 'eval/gittables/tables/tables/', ''), '.csv', '') AS table_file,
    file AS file_path
FROM glob('eval/gittables/tables/tables/GitTables_*.csv');

SELECT count(*) AS csv_files_found FROM csv_files;

-- For each annotated column, we need to extract data by column index.
-- Since column names are col0, col1, etc., column index N maps to column name 'col{N}'.
-- Strategy: read each table with all_varchar, UNPIVOT, filter to annotated columns.

-- Process all tables: read CSV → extract annotated columns → sample values
-- We use a CTE + LATERAL join to read each table dynamically

-- Since we can't easily UNPIVOT dynamically across varying-width CSVs,
-- let's use a more practical approach: read each CSV and extract specific columns
-- by constructing column names from the ground truth col_idx.

-- Build the column extraction: for each (table_file, col_idx) pair,
-- read the table and select col{col_idx}, then classify.

-- Most practical approach: read each table once with read_csv_auto(all_varchar),
-- and use UNPIVOT to get (col_name, value) pairs, then join with ground truth.

-- Let's process in batches. Start by reading a sample to validate:
CREATE OR REPLACE TABLE sample_unpivoted AS
WITH raw AS (
    SELECT * FROM read_csv('eval/gittables/tables/tables/GitTables_1501.csv',
                           header=true, all_varchar=true)
)
UNPIVOT raw ON COLUMNS(* EXCLUDE column00)
INTO NAME col_name VALUE col_value;

SELECT col_name, count(*) AS rows, count(col_value) AS non_null
FROM sample_unpivoted
GROUP BY col_name
ORDER BY col_name;

-- Now let's do this at scale. We'll read all tables, unpivot, join with ground truth.
-- The key challenge: varying number of columns per table.
-- Solution: read with union_by_name so DuckDB handles schema differences.

.print ''
.print '--- Reading all 1101 tables and unpivoting ---'

CREATE OR REPLACE TABLE all_columns AS
WITH raw AS (
    SELECT
        filename AS src_file,
        * EXCLUDE (filename)
    FROM read_csv('eval/gittables/tables/tables/GitTables_*.csv',
                  header=true, all_varchar=true, union_by_name=true, filename=true)
),
unpivoted AS (
    UNPIVOT raw ON COLUMNS(* EXCLUDE (src_file, column00))
    INTO NAME col_name VALUE col_value
)
SELECT
    replace(replace(src_file, 'eval/gittables/tables/tables/', ''), '.csv', '') AS table_file,
    CAST(regexp_extract(col_name, 'col(\d+)', 1) AS INTEGER) AS col_idx,
    col_value
FROM unpivoted
WHERE col_value IS NOT NULL
  AND length(col_value) > 0
  AND regexp_matches(col_name, '^col\d+$');

SELECT 'All columns extracted' AS step,
       count(*) AS total_values,
       count(DISTINCT table_file) AS tables,
       count(DISTINCT table_file || '_' || col_idx) AS unique_columns
FROM all_columns;

-- ═══════════════════════════════════════════════════════════════════════════════
-- 3. JOIN WITH GROUND TRUTH AND SAMPLE
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '--- Joining with ground truth annotations ---'

CREATE OR REPLACE TABLE annotated_values AS
SELECT
    a.table_file,
    a.col_idx,
    g.gt_label,
    g.ontology,
    a.col_value,
    row_number() OVER (PARTITION BY a.table_file, a.col_idx ORDER BY random()) AS sample_rank
FROM all_columns a
JOIN ground_truth g ON a.table_file = g.table_file AND a.col_idx = g.col_idx;

SELECT 'Annotated values' AS step,
       count(*) AS total_values,
       count(DISTINCT gt_label) AS unique_gt_labels,
       count(DISTINCT table_file || '_' || col_idx) AS annotated_columns
FROM annotated_values;

-- Sample up to 20 values per column for classification efficiency
CREATE OR REPLACE TABLE sampled_values AS
SELECT * FROM annotated_values WHERE sample_rank <= 20;

SELECT 'Sampled values' AS step,
       count(*) AS total_sampled
FROM sampled_values;

-- ═══════════════════════════════════════════════════════════════════════════════
-- 4. RUN FINETYPE INFERENCE
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '--- Running FineType classification ---'

CREATE OR REPLACE TABLE classified AS
SELECT
    table_file,
    col_idx,
    gt_label,
    ontology,
    col_value,
    finetype(col_value) AS ft_label,
    finetype_detail(col_value) AS ft_detail
FROM sampled_values;

SELECT 'Classification complete' AS step,
       count(*) AS classified_values
FROM classified;

-- ═══════════════════════════════════════════════════════════════════════════════
-- 5. AGGREGATE: Per-column majority vote
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '--- Per-column majority vote ---'

CREATE OR REPLACE TABLE column_predictions AS
WITH vote_counts AS (
    SELECT
        table_file,
        col_idx,
        gt_label,
        ontology,
        ft_label,
        count(*) AS votes,
        count(*) OVER (PARTITION BY table_file, col_idx) AS total_votes
    FROM classified
    GROUP BY table_file, col_idx, gt_label, ontology, ft_label
),
ranked AS (
    SELECT *,
           row_number() OVER (PARTITION BY table_file, col_idx ORDER BY votes DESC) AS rk
    FROM vote_counts
)
SELECT
    table_file,
    col_idx,
    gt_label,
    ontology,
    ft_label AS predicted_label,
    votes,
    total_votes,
    ROUND(votes * 100.0 / total_votes, 1) AS vote_pct
FROM ranked
WHERE rk = 1;

SELECT count(*) AS columns_with_predictions FROM column_predictions;

-- ═══════════════════════════════════════════════════════════════════════════════
-- 6. ANALYSIS: FineType domain distribution per GT label
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '                    FINETYPE × GITTABLES REPORT                   '
.print '═══════════════════════════════════════════════════════════════════'

-- 6a. FineType domain distribution for each ground truth label
.print ''
.print '--- FineType domain distribution per GT label ---'

SELECT
    gt_label,
    split_part(predicted_label, '.', 1) AS ft_domain,
    count(*) AS columns,
    ROUND(count(*) * 100.0 / sum(count(*)) OVER (PARTITION BY gt_label), 1) AS pct
FROM column_predictions
GROUP BY gt_label, ft_domain
HAVING count(*) >= 3
ORDER BY gt_label, columns DESC;

-- 6b. FineType detailed predictions for format-detectable types
.print ''
.print '--- Detailed FineType predictions for key semantic types ---'

SELECT
    gt_label,
    predicted_label,
    count(*) AS columns,
    ROUND(avg(vote_pct), 1) AS avg_vote_pct
FROM column_predictions
WHERE gt_label IN ('email', 'url', 'date', 'start date', 'end date', 'start time',
                   'end time', 'time', 'postal code', 'zip code', 'id',
                   'name', 'country', 'state', 'city', 'year', 'percentage',
                   'gender', 'age', 'price', 'duration', 'weight', 'height')
GROUP BY gt_label, predicted_label
ORDER BY gt_label, columns DESC;

-- 6c. Format-detectable accuracy: types where FineType should do well
.print ''
.print '--- Format-detectable types accuracy ---'

-- Create a mapping from GT labels to expected FineType domains
CREATE OR REPLACE TABLE type_mapping AS
SELECT * FROM (VALUES
    ('email',       'identity'),
    ('url',         'technology'),
    ('date',        'datetime'),
    ('start date',  'datetime'),
    ('end date',    'datetime'),
    ('start time',  'datetime'),
    ('end time',    'datetime'),
    ('time',        'datetime'),
    ('created',     'datetime'),
    ('updated',     'datetime'),
    ('year',        'datetime'),
    ('postal code', 'geography'),
    ('zip code',    'geography'),
    ('country',     'geography'),
    ('state',       'geography'),
    ('city',        'geography'),
    ('id',          'identity'),
    ('name',        'identity'),
    ('percentage',  'numeric'),
    ('age',         'numeric'),
    ('price',       'numeric'),
    ('weight',      'numeric'),
    ('height',      'numeric'),
    ('depth',       'numeric'),
    ('width',       'numeric'),
    ('length',      'numeric'),
    ('duration',    'numeric'),
    ('gender',      'identity'),
    ('author',      'identity'),
    ('description', 'representation'),
    ('title',       'representation'),
    ('abstract',    'representation'),
    ('comment',     'representation'),
    ('status',      'representation'),
    ('category',    'representation'),
    ('type',        'representation')
) AS t(gt_label, expected_ft_domain);

SELECT
    cp.gt_label,
    tm.expected_ft_domain,
    split_part(cp.predicted_label, '.', 1) AS actual_ft_domain,
    CASE WHEN split_part(cp.predicted_label, '.', 1) = tm.expected_ft_domain THEN 'match' ELSE 'mismatch' END AS domain_match,
    count(*) AS columns
FROM column_predictions cp
JOIN type_mapping tm ON cp.gt_label = tm.gt_label
GROUP BY cp.gt_label, tm.expected_ft_domain, actual_ft_domain, domain_match
ORDER BY cp.gt_label, columns DESC;

-- 6d. Domain-level accuracy summary
.print ''
.print '--- Domain-level accuracy for mapped types ---'

SELECT
    tm.expected_ft_domain,
    count(*) AS total_columns,
    sum(CASE WHEN split_part(cp.predicted_label, '.', 1) = tm.expected_ft_domain THEN 1 ELSE 0 END) AS correct,
    ROUND(sum(CASE WHEN split_part(cp.predicted_label, '.', 1) = tm.expected_ft_domain THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS accuracy_pct
FROM column_predictions cp
JOIN type_mapping tm ON cp.gt_label = tm.gt_label
GROUP BY tm.expected_ft_domain
ORDER BY total_columns DESC;

-- 6e. Overall summary
.print ''
.print '--- Overall statistics ---'

SELECT
    count(*) AS total_columns_evaluated,
    count(DISTINCT gt_label) AS unique_gt_types,
    count(DISTINCT predicted_label) AS unique_ft_predictions,
    count(DISTINCT split_part(predicted_label, '.', 1)) AS unique_ft_domains
FROM column_predictions;

-- Mapped types accuracy
SELECT
    'Mapped types (domain match)' AS metric,
    count(*) AS total,
    sum(CASE WHEN split_part(cp.predicted_label, '.', 1) = tm.expected_ft_domain THEN 1 ELSE 0 END) AS correct,
    ROUND(sum(CASE WHEN split_part(cp.predicted_label, '.', 1) = tm.expected_ft_domain THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS accuracy_pct
FROM column_predictions cp
JOIN type_mapping tm ON cp.gt_label = tm.gt_label;

-- 6f. Gaps: GT labels with no FineType coverage
.print ''
.print '--- GT labels with NO mapped FineType domain (gaps) ---'

SELECT
    cp.gt_label,
    count(*) AS columns,
    list(DISTINCT split_part(cp.predicted_label, '.', 1)) AS ft_domains_seen
FROM column_predictions cp
LEFT JOIN type_mapping tm ON cp.gt_label = tm.gt_label
WHERE tm.gt_label IS NULL
GROUP BY cp.gt_label
ORDER BY columns DESC;

-- 6g. FineType prediction confidence analysis
.print ''
.print '--- Classification confidence by GT label ---'

SELECT
    gt_label,
    count(*) AS values,
    ROUND(avg(CAST(json_extract_string(ft_detail, '$.confidence') AS DOUBLE)), 4) AS avg_confidence,
    ROUND(min(CAST(json_extract_string(ft_detail, '$.confidence') AS DOUBLE)), 4) AS min_confidence,
    ROUND(max(CAST(json_extract_string(ft_detail, '$.confidence') AS DOUBLE)), 4) AS max_confidence
FROM classified
GROUP BY gt_label
HAVING count(*) >= 20
ORDER BY avg_confidence DESC
LIMIT 25;

.print ''
.print '--- Evaluation complete ---'
