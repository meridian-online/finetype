-- Investigation: Technology domain regression (95.6% → 64.8%)
-- =============================================================================
-- Drills into which GT labels in the technology domain are failing,
-- what FineType predicts instead, and extracts sample misclassified values.
--
-- Usage: duckdb -unsigned < eval/gittables/investigate_tech.sql

SET threads = 8;
SET memory_limit = '4GB';

LOAD '${EXTENSION_PATH}';

.mode box

-- Load pre-extracted data
CREATE OR REPLACE TABLE metadata AS
SELECT * FROM read_csv('${EVAL_OUTPUT}/metadata.csv', auto_detect=true);

CREATE OR REPLACE TABLE column_values AS
SELECT * FROM read_parquet('${EVAL_OUTPUT}/column_values.parquet');

-- Flatten annotations
CREATE OR REPLACE TABLE ground_truth AS
SELECT
    m.topic,
    m.table_name,
    m.file_path,
    j.key AS col_name,
    j.value AS gt_label
FROM metadata m,
LATERAL (
    SELECT
        unnest(json_keys(m.annotations_json::JSON)) AS key,
        unnest(json_extract_string(m.annotations_json::JSON, '$.*'))
) j(key, value)
WHERE m.annotations_json IS NOT NULL AND m.annotations_json != '';

-- Classify values
CREATE OR REPLACE TABLE classified AS
SELECT
    topic,
    table_name,
    col_name,
    col_value,
    finetype(col_value) AS ft_label
FROM column_values;

-- Per-column majority vote
CREATE OR REPLACE TABLE column_predictions AS
WITH vote_counts AS (
    SELECT
        topic, table_name, col_name, ft_label,
        count(*) AS votes,
        sum(count(*)) OVER (PARTITION BY topic, table_name, col_name) AS total_votes
    FROM classified
    GROUP BY topic, table_name, col_name, ft_label
),
ranked AS (
    SELECT *, row_number() OVER (PARTITION BY topic, table_name, col_name ORDER BY votes DESC) AS rk
    FROM vote_counts
)
SELECT topic, table_name, col_name, ft_label AS predicted_label, votes, total_votes,
       ROUND(votes * 100.0 / total_votes, 1) AS vote_pct
FROM ranked WHERE rk = 1;

-- Join with ground truth
CREATE OR REPLACE TABLE eval_results AS
SELECT
    cp.topic, cp.table_name, cp.col_name,
    cp.predicted_label, cp.vote_pct,
    gt.gt_label,
    split_part(cp.predicted_label, '.', 1) AS ft_domain
FROM column_predictions cp
JOIN ground_truth gt ON cp.topic = gt.topic AND cp.table_name = gt.table_name AND cp.col_name = gt.col_name;

-- Domain mapping
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
    ('id',          'technology'),
    ('name',        'identity'),
    ('percentage',  'representation'),
    ('age',         'representation'),
    ('price',       'representation'),
    ('weight',      'representation'),
    ('height',      'representation'),
    ('depth',       'representation'),
    ('width',       'representation'),
    ('length',      'representation'),
    ('duration',    'datetime'),
    ('gender',      'identity'),
    ('author',      'identity'),
    ('description', 'representation'),
    ('title',       'representation'),
    ('abstract',    'representation'),
    ('comment',     'representation'),
    ('status',      'representation'),
    ('category',    'representation'),
    ('type',        'representation'),
    ('telephone',   'identity'),
    ('currency',    'representation'),
    ('latitude',    'geography'),
    ('longitude',   'geography'),
    ('address',     'geography'),
    ('brand',       'identity'),
    ('color',       'representation'),
    ('language',    'representation'),
    ('coordinates', 'geography')
) AS t(gt_label, expected_ft_domain);

-- ═══════════════════════════════════════════════════════════════════════════════
-- INVESTIGATION 1: Technology domain breakdown by GT label
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '  TECH DOMAIN: Per GT-label accuracy'
.print '═══════════════════════════════════════════════════════════════════'

SELECT
    er.gt_label,
    count(*) AS total,
    sum(CASE WHEN er.ft_domain = tm.expected_ft_domain THEN 1 ELSE 0 END) AS correct,
    ROUND(sum(CASE WHEN er.ft_domain = tm.expected_ft_domain THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS accuracy_pct,
    list(DISTINCT er.predicted_label ORDER BY er.predicted_label) FILTER (WHERE er.ft_domain != tm.expected_ft_domain) AS wrong_predictions
FROM eval_results er
JOIN type_mapping tm ON er.gt_label = tm.gt_label
WHERE tm.expected_ft_domain = 'technology'
GROUP BY er.gt_label
ORDER BY total DESC;

-- ═══════════════════════════════════════════════════════════════════════════════
-- INVESTIGATION 2: What does FineType predict for "id" columns?
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '  TECH DOMAIN: FineType predictions for gt_label=id'
.print '═══════════════════════════════════════════════════════════════════'

SELECT
    er.predicted_label,
    count(*) AS columns,
    ROUND(count(*) * 100.0 / sum(count(*)) OVER (), 1) AS pct,
    ROUND(avg(er.vote_pct), 1) AS avg_conf
FROM eval_results er
WHERE er.gt_label = 'id'
GROUP BY er.predicted_label
ORDER BY columns DESC
LIMIT 20;

-- ═══════════════════════════════════════════════════════════════════════════════
-- INVESTIGATION 3: What does FineType predict for "url" columns?
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '  TECH DOMAIN: FineType predictions for gt_label=url'
.print '═══════════════════════════════════════════════════════════════════'

SELECT
    er.predicted_label,
    count(*) AS columns,
    ROUND(count(*) * 100.0 / sum(count(*)) OVER (), 1) AS pct,
    ROUND(avg(er.vote_pct), 1) AS avg_conf
FROM eval_results er
WHERE er.gt_label = 'url'
GROUP BY er.predicted_label
ORDER BY columns DESC
LIMIT 20;

-- ═══════════════════════════════════════════════════════════════════════════════
-- INVESTIGATION 4: Sample misclassified "id" values
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '  SAMPLE: Misclassified "id" values (non-technology predictions)'
.print '═══════════════════════════════════════════════════════════════════'

SELECT
    c.ft_label,
    c.col_value,
    c.topic,
    c.col_name
FROM classified c
JOIN ground_truth gt ON c.topic = gt.topic AND c.table_name = gt.table_name AND c.col_name = gt.col_name
WHERE gt.gt_label = 'id'
  AND split_part(c.ft_label, '.', 1) != 'technology'
ORDER BY c.ft_label
LIMIT 40;

-- ═══════════════════════════════════════════════════════════════════════════════
-- INVESTIGATION 5: Sample misclassified "url" values
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '  SAMPLE: Misclassified "url" values (non-technology predictions)'
.print '═══════════════════════════════════════════════════════════════════'

SELECT
    c.ft_label,
    c.col_value,
    c.topic
FROM classified c
JOIN ground_truth gt ON c.topic = gt.topic AND c.table_name = gt.table_name AND c.col_name = gt.col_name
WHERE gt.gt_label = 'url'
  AND split_part(c.ft_label, '.', 1) != 'technology'
ORDER BY c.ft_label
LIMIT 40;

-- ═══════════════════════════════════════════════════════════════════════════════
-- INVESTIGATION 6: "id" column value patterns
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '  ID VALUE PATTERNS: What do ID columns actually contain?'
.print '═══════════════════════════════════════════════════════════════════'

-- Distribution of FineType predictions for individual values in id columns
SELECT
    c.ft_label,
    count(*) AS values,
    ROUND(count(*) * 100.0 / sum(count(*)) OVER (), 1) AS pct
FROM classified c
JOIN ground_truth gt ON c.topic = gt.topic AND c.table_name = gt.table_name AND c.col_name = gt.col_name
WHERE gt.gt_label = 'id'
GROUP BY c.ft_label
ORDER BY values DESC
LIMIT 20;

-- ═══════════════════════════════════════════════════════════════════════════════
-- INVESTIGATION 7: All domains — what FineType predicts for each GT label
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '  ALL DOMAINS: Confusion matrix (GT label → FT prediction)'
.print '═══════════════════════════════════════════════════════════════════'

SELECT
    tm.expected_ft_domain AS expected,
    er.gt_label,
    er.ft_domain AS predicted_domain,
    er.predicted_label,
    count(*) AS columns
FROM eval_results er
JOIN type_mapping tm ON er.gt_label = tm.gt_label
WHERE tm.expected_ft_domain = 'technology'
  AND er.ft_domain != 'technology'
GROUP BY ALL
ORDER BY columns DESC
LIMIT 30;

.print ''
.print '--- Investigation complete ---'
