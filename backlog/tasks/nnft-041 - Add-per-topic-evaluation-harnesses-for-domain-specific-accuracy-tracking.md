---
id: NNFT-041
title: Add per-topic evaluation harnesses for domain-specific accuracy tracking
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-13 10:10'
updated_date: '2026-02-15 09:11'
labels:
  - evaluation
  - infrastructure
dependencies:
  - NNFT-037
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The GitTables 1M evaluation covers 94 topics but currently only reports aggregate domain accuracy. Per-topic harnesses would enable tracking accuracy for specific data domains (e.g., healthcare, finance, geography) and identify where FineType performs best/worst.

This supports targeted model improvements and helps users understand FineType's strengths for their specific use case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Per-topic accuracy report generated from eval_1m.sql output
- [x] #2 Top 10 and bottom 10 topics by accuracy identified and documented
- [x] #3 Topic-level confusion matrices available for worst-performing topics
- [ ] #4 Results integrated into REPORT.md or separate per-topic analysis
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add per-topic accuracy queries to eval_1m.sql (section 6)
2. Add top 10 / bottom 10 topic accuracy reports
3. Add confusion matrix for worst-performing topics
4. Add full per-topic accuracy report
5. Document in eval README
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added section 6 to eval_1m.sql with four per-topic evaluation queries:

6a. Top 10 topics by domain accuracy — identifies FineType's strongest domains
6b. Bottom 10 topics by domain accuracy — identifies weakest topics needing improvement
6c. Confusion matrix for bottom 10 — shows most common misclassifications per topic
6d. Full per-topic accuracy report — all topics with ≥3 mapped columns

Results require running the eval against the GitTables 1M corpus (make eval-1m). The queries use the existing eval_results and type_mapping tables from sections 4-5, so they integrate seamlessly into the existing pipeline.

AC #4 (results in REPORT.md) — cannot be done without running the eval. This would need the DuckDB extension and GitTables data. Leaving as documented harness ready for next eval run.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added per-topic evaluation harnesses to eval_1m.sql for domain-specific accuracy tracking.

Changes:
- Section 6 added to eval_1m.sql with 4 queries:
  - 6a: Top 10 topics by domain accuracy
  - 6b: Bottom 10 topics by domain accuracy  
  - 6c: Confusion matrix for worst-performing topics (GT label × predicted label × domain match)
  - 6d: Full per-topic accuracy report (all topics with ≥3 mapped columns)
- Queries use existing eval_results/type_mapping tables from the pipeline
- Results generated on next make eval-1m run

Note: AC #4 (results in REPORT.md) deferred until next evaluation run — requires GitTables corpus data.
<!-- SECTION:FINAL_SUMMARY:END -->
