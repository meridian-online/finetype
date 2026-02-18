---
id: NNFT-097
title: Benchmark tiered-v2 on GitTables 1M evaluation
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 00:14'
updated_date: '2026-02-18 01:48'
labels:
  - evaluation
  - model
dependencies:
  - NNFT-040
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run the GitTables 1M stratified evaluation benchmark (established in NNFT-040) with the tiered-v2 model and compare against the flat CharCNN v6/v7 baseline. This provides a large-scale real-world validation beyond the 20-dataset profile eval.

The eval infrastructure exists in eval/eval_1m.sql with per-topic harnesses (NNFT-041). Need to run column-mode evaluation with tiered model and compare domain/topic accuracy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 GitTables 1M evaluation completed with tiered-v2 model
- [x] #2 Results compared against flat CharCNN baseline from NNFT-082
- [x] #3 Per-topic accuracy breakdown generated
- [x] #4 Regression analysis identifies any topics where tiered is worse
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Run GitTables 1M eval with the OLD flat extension (current finetype-duckdb crate, CharCNN v6) — baseline
2. Run GitTables 1M eval with the NEW tiered extension (duckdb-finetype repo, tiered-v2) — test
3. Compare headline accuracy, per-type, and per-topic results
4. Generate regression analysis (topics where tiered is worse)
5. Also run the 20-dataset profile eval with tiered model via CLI for cross-validation
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Profile eval (20 datasets, 206 annotations) completed with tiered-v2 via CLI:
- Format-detectable label accuracy: **72.6%** (82/113 columns)
- Format-detectable domain accuracy: **84.1%** (95/113 columns)
- Partially-detectable label accuracy: 27.9% (19/68)
- Partially-detectable domain accuracy: 51.5% (35/68)

This matches the NNFT-090 baseline. Now running GitTables 1M eval (774,350 values across 4,380 tables, 45,428 columns)."

GitTables 1M eval completed with tiered-v2 (via duckdb-finetype extension v0.2.0):
- 774,350 values classified in 809 seconds (2.6x slower than flat's 307s)
- Format-detectable: **40.9% label, 80.9% domain** (4,482 cols) — up from 35.2%/57.2% flat
- All mapped: 8.0% label, 49.4% domain (23,466 cols) — domain down from 62.3% flat
- Identity domain: 79.4% (up from 35.4%) — biggest win
- Representation domain: 44.3% (down from 70.3%) — semantic-only shift
- 118 unique types detected (vs 157 flat)

Regression: sentence type went from 96.6% to 0% — tiered T1 routing bug.
Performance: 2.6x slower tracked in NNFT-098.

Updated eval/gittables/REPORT.md with full v0.1.7 tiered-v2 section.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Benchmarked tiered-v2 model on GitTables 1M evaluation (774,350 values, 4,380 tables, 45,428 columns) and compared against flat CharCNN v6 baseline.

**Headline: +23.7% domain accuracy on format-detectable types (57.2% → 80.9%)**

Key results:
- Format-detectable label accuracy: 35.2% → 40.9% (+5.7%)
- Format-detectable domain accuracy: 57.2% → 80.9% (+23.7%)
- Identity domain: 35.4% → 79.4% (+44.0%) — standout improvement
- Technology domain: 91.8% → 92.0% (stable)
- Geography domain: 5.5% → 16.1% (+10.6%)

Regressions identified:
- All-mapped domain accuracy: 62.3% → 49.4% (semantic-only types shifted from 73.0% → 47.8%)
- Representation domain: 70.3% → 44.3% (dominated by semantic-only columns)
- Sentence type: 96.6% → 0% label recall (tiered routing bug)
- Performance: 307s → 809s (2.6x slower, tracked in NNFT-098)
- Type coverage: 157 → 118 unique types detected

Updated eval/gittables/REPORT.md with complete v0.1.7 tiered-v2 evaluation section including baseline comparison, domain breakdown, per-type metrics, misclassification patterns, regression analysis, and per-topic accuracy for all 94 topics.

Also ran profile eval (20 datasets): 72.6% label / 84.1% domain accuracy on format-detectable types."
<!-- SECTION:FINAL_SUMMARY:END -->
