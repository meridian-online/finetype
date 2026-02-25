-- SOTAB CTA Evaluation: FineType CLI pipeline (NNFT-130)
-- =============================================================================
-- Scores CLI batch predictions against ground truth using the same schema
-- mapping and accuracy metrics as eval_sotab.sql, but reads pre-computed
-- predictions from cli_predictions.csv instead of running the DuckDB extension.
--
-- Pipeline:
--   1. prepare_values.py  — Extract column values from SOTAB JSON tables
--   2. eval_cli.py        — Classify via CLI batch mode (tiered + disambiguation)
--   3. This script        — Score predictions against ground truth
--
-- Usage:
--   make eval-sotab-cli                                 # Via Makefile (recommended)
--   SOTAB_DIR=~/datasets/sotab/cta SOTAB_SPLIT=validation \
--     envsubst '$SOTAB_DIR $SOTAB_SPLIT' < eval/sotab/eval_cli.sql | duckdb
--
-- Path variables (substituted by envsubst via Makefile):
--   ${SOTAB_DIR}       — SOTAB CTA data directory
--   ${SOTAB_SPLIT}     — Dataset split (validation or test)
--
-- Prerequisites:
--   1. Column values extracted: make eval-sotab-values
--   2. CLI predictions: eval/sotab/eval_cli.py
--   3. Schema mapping: eval/sotab/sotab_schema_mapping.csv

SET threads = 8;
SET memory_limit = '4GB';

.mode box
.timer on

-- ═══════════════════════════════════════════════════════════════════════════════
-- 1. LOAD CLI PREDICTIONS AND GROUND TRUTH
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '          SOTAB CTA CLI — LOADING DATA                           '
.print '═══════════════════════════════════════════════════════════════════'

CREATE OR REPLACE TABLE column_predictions AS
SELECT
    table_name,
    col_index,
    gt_label,
    predicted_label,
    confidence,
    samples_used,
    disambiguation_rule
FROM read_csv('${SOTAB_DIR}/${SOTAB_SPLIT}/cli_predictions.csv', auto_detect=true);

.print ''
.print '--- CLI predictions loaded ---'
SELECT
    count(*) AS total_columns,
    count(DISTINCT predicted_label) AS unique_ft_predictions,
    count(DISTINCT gt_label) AS unique_gt_labels,
    sum(CASE WHEN disambiguation_rule IS NOT NULL AND disambiguation_rule != '' THEN 1 ELSE 0 END) AS disambiguated
FROM column_predictions;

.print ''
.print '--- Top 20 ground truth labels by column count ---'
SELECT
    gt_label,
    count(*) AS columns
FROM column_predictions
GROUP BY gt_label
ORDER BY columns DESC
LIMIT 20;

-- ═══════════════════════════════════════════════════════════════════════════════
-- 2. APPLY SCHEMA MAPPING
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '          SCHEMA MAPPING & ACCURACY                              '
.print '═══════════════════════════════════════════════════════════════════'

CREATE OR REPLACE TABLE schema_mapping AS
SELECT * FROM read_csv('eval/sotab/sotab_schema_mapping.csv', auto_detect=true);

.print ''
.print '--- Schema mapping loaded ---'
SELECT
    count(*) AS total_mappings,
    count(*) FILTER (match_quality = 'direct') AS direct,
    count(*) FILTER (match_quality = 'close') AS close,
    count(*) FILTER (match_quality = 'partial') AS partial,
    count(*) FILTER (match_quality = 'semantic_only') AS semantic_only
FROM schema_mapping;

CREATE OR REPLACE TABLE eval_results AS
SELECT
    cp.table_name,
    cp.col_index,
    cp.predicted_label,
    cp.confidence,
    cp.disambiguation_rule,
    cp.gt_label,
    split_part(cp.predicted_label, '.', 1) AS ft_domain,
    sm.finetype_label AS expected_ft_label,
    sm.finetype_domain AS expected_ft_domain,
    sm.match_quality,
    -- Label-level match
    CASE
        WHEN sm.finetype_label IS NOT NULL AND sm.finetype_label != ''
             AND cp.predicted_label = sm.finetype_label
        THEN true
        ELSE false
    END AS label_match,
    -- Domain-level match
    CASE
        WHEN sm.finetype_domain IS NOT NULL AND sm.finetype_domain != ''
             AND split_part(cp.predicted_label, '.', 1) = sm.finetype_domain
        THEN true
        ELSE false
    END AS domain_match,
    -- Detectability tier
    CASE
        WHEN sm.match_quality IN ('direct', 'close') THEN 'format_detectable'
        WHEN sm.match_quality = 'partial' THEN 'partially_detectable'
        WHEN sm.match_quality = 'semantic_only' THEN 'semantic_only'
        ELSE 'unmapped'
    END AS detectability
FROM column_predictions cp
LEFT JOIN schema_mapping sm ON cp.gt_label = sm.sotab_label;

SELECT
    count(*) AS total_columns,
    count(DISTINCT gt_label) AS unique_gt_labels,
    count(DISTINCT predicted_label) AS unique_ft_predictions,
    count(*) FILTER (match_quality IS NOT NULL) AS mapped,
    count(*) FILTER (match_quality IS NULL) AS unmapped
FROM eval_results;

-- ═══════════════════════════════════════════════════════════════════════════════
-- 3. HEADLINE ACCURACY
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '          HEADLINE ACCURACY                                       '
.print '═══════════════════════════════════════════════════════════════════'

-- 3a. Accuracy by detectability tier
.print ''
.print '--- Accuracy by detectability tier ---'
SELECT
    detectability,
    count(*) AS columns,
    sum(CASE WHEN label_match THEN 1 ELSE 0 END) AS label_correct,
    ROUND(sum(CASE WHEN label_match THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS label_accuracy_pct,
    sum(CASE WHEN domain_match THEN 1 ELSE 0 END) AS domain_correct,
    ROUND(sum(CASE WHEN domain_match THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS domain_accuracy_pct
FROM eval_results
WHERE match_quality IS NOT NULL
GROUP BY detectability
ORDER BY
    CASE detectability
        WHEN 'format_detectable' THEN 1
        WHEN 'partially_detectable' THEN 2
        WHEN 'semantic_only' THEN 3
        ELSE 4
    END;

-- 3b. Headline: format-detectable types (direct + close)
.print ''
.print '--- HEADLINE: Format-detectable accuracy (direct + close) ---'
SELECT
    'Format-detectable (direct + close)' AS metric,
    count(*) AS columns,
    sum(CASE WHEN label_match THEN 1 ELSE 0 END) AS label_correct,
    ROUND(sum(CASE WHEN label_match THEN 1 ELSE 0 END) * 100.0 / NULLIF(count(*), 0), 1) AS label_accuracy_pct,
    sum(CASE WHEN domain_match THEN 1 ELSE 0 END) AS domain_correct,
    ROUND(sum(CASE WHEN domain_match THEN 1 ELSE 0 END) * 100.0 / NULLIF(count(*), 0), 1) AS domain_accuracy_pct
FROM eval_results
WHERE detectability = 'format_detectable';

-- 3c. Overall mapped accuracy
.print ''
.print '--- Overall mapped accuracy (all mapped types) ---'
SELECT
    'All mapped types' AS metric,
    count(*) AS total,
    sum(CASE WHEN label_match THEN 1 ELSE 0 END) AS label_correct,
    ROUND(sum(CASE WHEN label_match THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS label_accuracy_pct,
    sum(CASE WHEN domain_match THEN 1 ELSE 0 END) AS domain_correct,
    ROUND(sum(CASE WHEN domain_match THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS domain_accuracy_pct
FROM eval_results
WHERE match_quality IS NOT NULL;

-- 3d. Direct match accuracy only (strongest signal)
.print ''
.print '--- Direct match accuracy only (exact type correspondence) ---'
SELECT
    'Direct matches only' AS metric,
    count(*) AS total,
    sum(CASE WHEN label_match THEN 1 ELSE 0 END) AS label_correct,
    ROUND(sum(CASE WHEN label_match THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS label_accuracy_pct,
    sum(CASE WHEN domain_match THEN 1 ELSE 0 END) AS domain_correct,
    ROUND(sum(CASE WHEN domain_match THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS domain_accuracy_pct
FROM eval_results
WHERE match_quality = 'direct';

-- ═══════════════════════════════════════════════════════════════════════════════
-- 4. PER-LABEL METRICS
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '          PER-LABEL METRICS                                       '
.print '═══════════════════════════════════════════════════════════════════'

-- 4a. Per GT label accuracy (format-detectable)
.print ''
.print '--- Per GT label accuracy (direct + close) ---'
SELECT
    gt_label,
    match_quality,
    expected_ft_label,
    count(*) AS total,
    sum(CASE WHEN label_match THEN 1 ELSE 0 END) AS label_correct,
    ROUND(sum(CASE WHEN label_match THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS label_recall_pct,
    sum(CASE WHEN domain_match THEN 1 ELSE 0 END) AS domain_correct,
    ROUND(sum(CASE WHEN domain_match THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS domain_recall_pct,
    ROUND(avg(confidence), 3) AS avg_confidence
FROM eval_results
WHERE match_quality IN ('direct', 'close')
GROUP BY gt_label, match_quality, expected_ft_label
ORDER BY total DESC;

-- 4b. All labels including partial and semantic
.print ''
.print '--- All labels accuracy (all mapped) ---'
SELECT
    gt_label,
    match_quality,
    expected_ft_label,
    count(*) AS total,
    sum(CASE WHEN label_match THEN 1 ELSE 0 END) AS label_correct,
    ROUND(sum(CASE WHEN label_match THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS label_recall_pct,
    sum(CASE WHEN domain_match THEN 1 ELSE 0 END) AS domain_correct,
    ROUND(sum(CASE WHEN domain_match THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS domain_recall_pct,
    ROUND(avg(confidence), 3) AS avg_confidence
FROM eval_results
WHERE match_quality IS NOT NULL
GROUP BY gt_label, match_quality, expected_ft_label
ORDER BY total DESC;

-- 4c. Domain-level accuracy breakdown
.print ''
.print '--- Domain-level accuracy by expected domain ---'
SELECT
    expected_ft_domain,
    count(*) AS total_columns,
    sum(CASE WHEN domain_match THEN 1 ELSE 0 END) AS correct,
    ROUND(sum(CASE WHEN domain_match THEN 1 ELSE 0 END) * 100.0 / count(*), 1) AS accuracy_pct
FROM eval_results
WHERE expected_ft_domain IS NOT NULL AND expected_ft_domain != ''
GROUP BY expected_ft_domain
ORDER BY total_columns DESC;

-- ═══════════════════════════════════════════════════════════════════════════════
-- 5. MISCLASSIFICATION ANALYSIS
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '          MISCLASSIFICATION ANALYSIS                              '
.print '═══════════════════════════════════════════════════════════════════'

-- 5a. Top misclassification patterns (format-detectable)
.print ''
.print '--- Top misclassification patterns (format-detectable, wrong label) ---'
SELECT
    gt_label,
    expected_ft_label,
    predicted_label AS actual_prediction,
    match_quality,
    count(*) AS occurrences,
    ROUND(avg(confidence), 3) AS avg_conf
FROM eval_results
WHERE NOT label_match
  AND match_quality IN ('direct', 'close')
  AND expected_ft_label IS NOT NULL
  AND expected_ft_label != ''
GROUP BY gt_label, expected_ft_label, predicted_label, match_quality
ORDER BY occurrences DESC
LIMIT 25;

-- 5b. Wrong-domain errors (format-detectable)
.print ''
.print '--- Wrong domain errors (format-detectable) ---'
SELECT
    gt_label,
    expected_ft_domain,
    ft_domain AS predicted_domain,
    predicted_label,
    count(*) AS occurrences
FROM eval_results
WHERE NOT domain_match
  AND match_quality IN ('direct', 'close')
  AND expected_ft_domain IS NOT NULL AND expected_ft_domain != ''
GROUP BY gt_label, expected_ft_domain, ft_domain, predicted_label
ORDER BY occurrences DESC
LIMIT 20;

-- 5c. Semantic gap summary
.print ''
.print '--- Semantic gap summary (FineType cannot detect by design) ---'
SELECT
    gt_label,
    count(*) AS columns,
    list(DISTINCT predicted_label ORDER BY predicted_label)[:5] AS ft_predictions,
    ROUND(avg(confidence), 3) AS avg_confidence
FROM eval_results
WHERE detectability = 'semantic_only'
GROUP BY gt_label
HAVING count(*) >= 3
ORDER BY columns DESC
LIMIT 20;

-- ═══════════════════════════════════════════════════════════════════════════════
-- 6. DISTRIBUTION & COVERAGE
-- ═══════════════════════════════════════════════════════════════════════════════

.print ''
.print '═══════════════════════════════════════════════════════════════════'
.print '          DISTRIBUTION & COVERAGE                                 '
.print '═══════════════════════════════════════════════════════════════════'

-- 6a. FineType domain distribution
.print ''
.print '--- FineType domain distribution (all predicted columns) ---'
SELECT
    split_part(predicted_label, '.', 1) AS ft_domain,
    count(*) AS columns,
    ROUND(count(*) * 100.0 / sum(count(*)) OVER (), 1) AS pct
FROM column_predictions
GROUP BY ft_domain
ORDER BY columns DESC;

-- 6b. Top 30 FineType predictions
.print ''
.print '--- Top 30 FineType predictions ---'
SELECT
    predicted_label,
    count(*) AS columns,
    ROUND(count(*) * 100.0 / sum(count(*)) OVER (), 1) AS pct,
    ROUND(avg(confidence), 3) AS avg_confidence
FROM column_predictions
GROUP BY predicted_label
ORDER BY columns DESC
LIMIT 30;

-- 6c. Low confidence predictions
.print ''
.print '--- Low confidence predictions (confidence < 0.6) ---'
SELECT
    cp.predicted_label,
    cp.gt_label,
    count(*) AS low_conf_columns,
    ROUND(avg(cp.confidence), 3) AS avg_confidence
FROM column_predictions cp
WHERE cp.confidence < 0.6
GROUP BY cp.predicted_label, cp.gt_label
ORDER BY low_conf_columns DESC
LIMIT 20;

-- 6d. Disambiguation rule distribution
.print ''
.print '--- Disambiguation rule distribution ---'
SELECT
    COALESCE(NULLIF(disambiguation_rule, ''), '(none)') AS rule,
    count(*) AS columns,
    ROUND(count(*) * 100.0 / sum(count(*)) OVER (), 1) AS pct
FROM column_predictions
GROUP BY rule
ORDER BY columns DESC
LIMIT 20;

-- 6e. Throughput summary
.print ''
.print '--- Throughput summary ---'
SELECT
    count(*) AS total_columns,
    count(DISTINCT table_name) AS total_tables,
    count(DISTINCT gt_label) AS unique_gt_labels
FROM column_predictions;

.print ''
.print '--- Evaluation complete ---'
