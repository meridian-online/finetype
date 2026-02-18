---
id: NNFT-098
title: Investigate tiered model performance and optimize for large dataset evaluation
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 01:40'
updated_date: '2026-02-18 06:08'
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
- [x] #1 Throughput benchmarked: flat vs tiered (values/sec, both single-threaded and batched)
- [x] #2 Tier-level timing breakdown identifies where time is spent
- [x] #3 At least one optimization implemented or documented with measured impact
- [x] #4 CLI --model-type guidance documents performance/accuracy tradeoff
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Optimized tiered inference throughput by 30x and added CLI performance instrumentation.

## Changes

### Batched tier processing (c2a3bf3)
- Rewrote `classify_batch` in tiered.rs: group-then-batch processing replaces per-sample T1/T2 forwarding
- CLI infer command now processes in chunks of 128 (was per-value) for all model types
- **Throughput: 17 → 580 val/sec (tiered), 1500 val/sec (flat)**

### Performance instrumentation (fa29ed4)
- `--bench` flag on infer command: prints throughput + tier-level timing breakdown to stderr
- `TierTiming` struct + `classify_batch_timed()` method for per-tier measurement
- `--model-type` help text documents throughput tradeoff (~600 vs ~1500 val/sec)

### Column mode fix (fa29ed4)
- `--mode column` now works with all model types (was char-cnn only), fixing CI smoke test failure

## Benchmark results (10K values)
| Model | Throughput | T0 share | T1 share | T2 share |
|-------|-----------|----------|----------|----------|
| Tiered | ~580 val/sec | 37% | 32% | 31% |
| Flat | ~1500 val/sec | — | — | — |

Tier time is evenly distributed across T0/T1/T2 — no single bottleneck tier.

## Commits
- c2a3bf3: Batch tiered inference (30x improvement)
- fa29ed4: --bench flag, tier timing, column mode fix
<!-- SECTION:FINAL_SUMMARY:END -->
