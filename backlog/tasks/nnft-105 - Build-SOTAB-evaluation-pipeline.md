---
id: NNFT-105
title: Build SOTAB evaluation pipeline
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 09:17'
updated_date: '2026-02-18 09:58'
labels:
  - evaluation
  - pipeline
dependencies:
  - NNFT-104
references:
  - eval/gittables/eval_1m.sql
  - eval/gittables/prepare_1m_values.py
  - eval/gittables/extract_metadata_1m.py
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create an evaluation pipeline for SOTAB analogous to the existing GitTables 1M pipeline. SOTAB tables are in JSON format (from web scraping), so the pipeline needs to handle JSON→columnar extraction, value sampling, FineType classification, and accuracy measurement against the CTA ground truth.

The pipeline should support both DuckDB extension and CLI inference paths, and produce comparable metrics to the GitTables eval (domain accuracy, label accuracy, per-detectability-tier breakdown, confidence analysis).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Python script to extract column values from SOTAB JSON tables (sample up to 20 non-null values per column)
- [x] #2 Column values exported to parquet for DuckDB classification
- [x] #3 DuckDB eval SQL script that classifies values, performs majority vote, and compares against SOTAB CTA ground truth
- [x] #4 Metrics output: domain accuracy, label accuracy, per-detectability-tier breakdown
- [x] #5 Per-label accuracy table showing top performers and worst misclassifications
- [x] #6 Makefile target: make eval-sotab
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Create eval/sotab/prepare_values.py — extract column values from SOTAB JSON tables
   - Read CTA GT CSV to know which columns have annotations
   - Read corresponding .json.gz table files
   - Sample up to 20 non-null values per annotated column
   - Output column_values.parquet with: table_name, col_index, col_value

2. Create eval/sotab/eval_sotab.sql — DuckDB eval script
   - Load column_values.parquet
   - Classify with finetype()
   - Majority vote per column
   - Join with ground truth CSV
   - Apply sotab_schema_mapping.csv
   - Compute headline accuracy, per-tier breakdown, per-label metrics

3. Add Makefile targets: eval-sotab-values, eval-sotab

4. Test on validation set first, then run on test set
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Implementation

**prepare_values.py** (eval/sotab/prepare_values.py):
- Reads SOTAB ground truth CSV to identify annotated columns
- Opens gzipped JSON table files, extracts values by column index
- Samples up to 20 non-null values per annotated column
- Outputs column_values.parquet with table_name, col_index, gt_label, col_value
- Supports --split (validation/test), --gt-file, --output, --sotab-dir args

**eval_sotab.sql** (eval/sotab/eval_sotab.sql):
- Loads column_values.parquet (includes embedded GT labels)
- Classifies all values with finetype() DuckDB extension
- Majority vote per column for prediction
- Joins with sotab_schema_mapping.csv for match quality tiers
- Computes headline, per-tier, per-label, domain-level accuracy
- Misclassification analysis and semantic gap summary

**Makefile targets:**
- eval-sotab-values: extract values from validation set
- eval-sotab: run DuckDB classification and scoring
- eval-sotab-all: full pipeline

**Bug fix:** Fixed vote_pct window function in both eval_1m.sql and eval_sotab.sql — was using count(*) OVER instead of sum(count(*)) OVER, inflating confidence scores.

**First run results (validation set, char-cnn-v7):**
- 282,278 values classified from 16,765 columns across 5,728 tables
- Classification time: 469s
- Format-detectable: 25.4% label accuracy, 53.7% domain accuracy
- All mapped: 18.5% label, 44.6% domain
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Built complete SOTAB CTA evaluation pipeline matching the GitTables eval structure.

**New files:**
- `eval/sotab/prepare_values.py` — extracts and samples column values from SOTAB JSON tables
- `eval/sotab/eval_sotab.sql` — DuckDB eval script with classification, voting, and accuracy measurement

**Makefile targets:** `eval-sotab-values`, `eval-sotab`, `eval-sotab-all`

**Bug fix:** Corrected vote_pct window function in both `eval_1m.sql` and `eval_sotab.sql` — was `count(*) OVER` (counting groups) instead of `sum(count(*)) OVER` (summing votes), which inflated confidence metrics.

**First results (validation, char-cnn-v7, 16,765 columns):**
- Format-detectable: 25.4% label, 53.7% domain
- Direct matches only: higher accuracy on types like URL (76%), currency (95.3%), Duration
- Top misclassifications: Organization/Person names classified as text types, weight values as decimal_number
<!-- SECTION:FINAL_SUMMARY:END -->
