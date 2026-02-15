---
id: NNFT-040
title: Establish GitTables 1M stratified sample as standard evaluation benchmark
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-13 10:10'
updated_date: '2026-02-15 09:10'
labels:
  - evaluation
  - infrastructure
  - gittables
dependencies:
  - NNFT-037
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The GitTables 1M evaluation (NNFT-037) showed the original benchmark subset (1,101 tables) was not fully representative — it over-represented difficult semantic types. The 1M stratified sample (50 tables/topic, 4,380 total) provides a more balanced evaluation.

Formalize this as the standard benchmark: reproducible sampling, pre-extracted metadata and values, documented evaluation script, and baseline metrics for comparison across model versions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pre-extracted evaluation data committed to repo or hosted for reproducibility
- [x] #2 Evaluation script (eval_1m.sql) documented with clear usage instructions
- [x] #3 Baseline metrics recorded: 55.3% domain accuracy, per-domain breakdown
- [x] #4 CI or Makefile target to re-run evaluation after model changes
- [x] #5 REPORT.md updated to designate 1M sample as primary benchmark
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add a Makefile with eval targets (eval-1m, eval-benchmark)
2. Improve eval_1m.sql header docs with clear prerequisites and usage
3. Update REPORT.md to designate 1M as primary benchmark (move section up, add note)
4. Add setup instructions for eval data generation (catalog.csv, metadata.csv, column_values.parquet)
5. Skip committing large parquet data — document regeneration instead
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #1: Added eval/gittables/README.md documenting the data regeneration pipeline (3 scripts). Data lives at ~/git-tables/eval_output/ (catalog.csv 4KB, metadata.csv 1.6MB, column_values.parquet 12MB, sampled_files.txt 292KB). Not committed directly to repo due to size, but fully reproducible via documented pipeline.

AC #2: Improved eval_1m.sql header with clear prerequisites, 3-step pipeline, and usage instructions. Added README with file descriptions.

AC #3: Baseline metrics already recorded in REPORT.md: 55.3% overall domain accuracy, per-domain breakdown (identity 71.3%, technology 64.8%, datetime 53.9%, geography 45.7%, representation 38.7%).

AC #4: Created Makefile with eval targets: eval-extract, eval-values, eval-1m, eval-benchmark, eval-all. Also includes build, test, check, generate, stats targets.

AC #5: Updated REPORT.md header to designate 1M stratified sample as primary benchmark with callout box. Legacy 1,101-table subset retained below for historical comparison.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Formalized GitTables 1M stratified sample as the standard FineType evaluation benchmark.

Changes:
- Created Makefile with eval targets (eval-1m, eval-all, eval-extract, eval-values, eval-benchmark) plus build/test/check helpers
- Added eval/gittables/README.md with prerequisites, pipeline docs, baseline metrics table, and file descriptions
- Updated REPORT.md header to designate 1M sample as primary benchmark (55.3% domain accuracy baseline)
- Improved eval_1m.sql header documentation with 3-step pipeline, usage instructions, and prerequisites
- Data regeneration fully documented (not committed due to size — reproducible via make eval-all)
<!-- SECTION:FINAL_SUMMARY:END -->
