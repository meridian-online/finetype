---
id: NNFT-082
title: Run expanded GitTables 1M evaluation with CharCNN v6
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-16 10:49'
updated_date: '2026-02-16 22:02'
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
- [ ] #1 GitTables 1M eval runs successfully with v6 model/DuckDB extension
- [ ] #2 Results compared against v0.1.0 baseline (55.3%)
- [ ] #3 Per-domain accuracy breakdown reported
- [ ] #4 Per-type accuracy for format-detectable types reported
- [ ] #5 Regressions from v5→v6 identified and documented
- [ ] #6 REPORT.md updated with v0.1.5 results section
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
