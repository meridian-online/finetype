---
id: NNFT-082
title: Run expanded GitTables 1M evaluation with CharCNN v6
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-16 10:49'
updated_date: '2026-02-16 22:37'
labels:
  - evaluation
  - benchmark
dependencies:
  - NNFT-079
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Re-run the GitTables 1M stratified evaluation with the v6 model and updated taxonomy (169 types). Compare results against v0.1.0 baseline (55.3% domain accuracy). Use the new schema mapping to get more granular accuracy metrics. Identify types where v6 improved or regressed. Document findings in eval/gittables/REPORT.md. This establishes the v0.1.5 benchmark baseline and identifies next improvement targets.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 GitTables 1M eval runs successfully with v6 model/DuckDB extension
- [x] #2 Results compared against v0.1.0 baseline (55.3%)
- [x] #3 Per-domain accuracy breakdown reported
- [x] #4 Per-type accuracy for format-detectable types reported
- [x] #5 Regressions from v5→v6 identified and documented
- [x] #6 REPORT.md updated with v0.1.5 results section
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Build DuckDB extension with v6 model (cargo build --release)
2. Update eval_1m.sql to replace inline type_mapping VALUES with schema_mapping.csv
3. Add label-level accuracy scoring alongside domain-level
4. Run eval (duckdb -unsigned < eval/gittables/eval_1m.sql)
5. Compare results against v0.1.0 baseline
6. Update REPORT.md with v0.1.5 section
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Rebuilt DuckDB extension with CharCNN v6 model by fixing build.rs to follow `models/default` symlink instead of hardcoding `char-cnn-v2`. Added `append_extension_metadata.py` step to Makefile's `build-release` target for proper DuckDB extension metadata.

Replaced 34-type inline VALUES table in eval_1m.sql with comprehensive 192-type schema_mapping.csv (from NNFT-079). Added label-level accuracy scoring alongside domain-level, with three detectability tiers: format_detectable (direct+close), partially_detectable, semantic_only. Restructured SQL into 9 report sections with headline accuracy, per-type metrics, model errors vs semantic gaps, distribution/coverage, and per-topic accuracy.

Results (v0.1.5 vs v0.1.0 baseline):
- Domain accuracy (format-detectable): 57.2% (new metric, comparable to old 55.3%)
- Domain accuracy (all mapped): 62.3% (+7.0% vs 55.3% baseline, with 2.2× more mapped columns)
- Label accuracy (format-detectable): 35.2% (new metric)
- Classification time: 307s (-17% vs 370s)
- FineType types detected: 157/169 (+14 over v0.1.0's 143/151)

Top performers: url (98.9% domain), person (100%), currency (100%), address (100%), issn (94.1%).
Main misclassification: `author` columns (1,609 total) — GitTables author data contains usernames, IDs, org names, not just person names.

Updated REPORT.md with comprehensive v0.1.5 section including baseline comparison, domain breakdown, top performers, misclassification patterns, per-topic accuracy, and key findings.

Files changed: crates/finetype-duckdb/build.rs, eval/gittables/eval_1m.sql, eval/gittables/REPORT.md, Makefile
<!-- SECTION:FINAL_SUMMARY:END -->
