---
id: NNFT-098
title: Investigate tiered model performance and optimize for large dataset evaluation
status: To Do
assignee: []
created_date: '2026-02-18 01:40'
labels:
  - performance
  - model
  - usability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The tiered-v2 model uses 34 CharCNN models in a T0→T1→T2 cascade, which significantly increases inference latency compared to the flat single-model approach. During the GitTables 1M evaluation (774K values), classification takes 10+ minutes at 263% CPU utilization — substantially slower than the flat model which processed the same dataset in under a minute.

This has direct usability implications:
- `finetype profile` on large CSVs will be noticeably slower
- The DuckDB extension's `finetype()` function on large tables is impacted
- Batch operations (data profiling pipelines) may become bottlenecked

Need to investigate:
1. Actual throughput comparison: flat vs tiered (values/sec)
2. Where time is spent in the tiered pipeline (T0 vs T1 vs T2 breakdown)
3. Optimization opportunities: caching T0/T1 results, batch inference at each tier, parallel tier execution
4. Whether the accuracy improvement justifies the performance cost for different use cases
5. Consider offering both model types with clear guidance on when to use each
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Throughput benchmarked: flat vs tiered (values/sec, both single-threaded and batched)
- [ ] #2 Tier-level timing breakdown identifies where time is spent
- [ ] #3 At least one optimization implemented or documented with measured impact
- [ ] #4 CLI --model-type guidance documents performance/accuracy tradeoff
<!-- AC:END -->
