---
id: NNFT-089
title: Make tiered-v2 the default inference model
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-17 22:44'
updated_date: '2026-02-17 22:57'
labels:
  - model
  - architecture
dependencies:
  - NNFT-084
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Switch the default model from char-cnn-v5 (flat) to tiered-v2. This requires:
1. Updating models/default symlink to point to tiered-v2
2. Updating build.rs to embed tiered model artifacts (tier_graph.json + all tier directories)
3. Changing default model-type from flat to tiered in CLI
4. Updating tests that depend on flat model predictions

The tiered model achieves 72.6% label accuracy on format-detectable types vs 68.1% for flat — a clear improvement that justifies making it the default.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 models/default symlink points to tiered-v2
- [x] #2 build.rs embeds tiered model artifacts (tier_graph.json + all tier subdirectories)
- [x] #3 CLI defaults to --model-type tiered when no flag specified
- [x] #4 All existing tests updated and passing with tiered model
- [x] #5 finetype infer produces correct results without explicit --model-type flag
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Switched default model from char-cnn-v7 (flat) to tiered-v2. Updated models/default symlink. build.rs now detects tiered model via tier_graph.json and generates embedded lookup function for all 34 tier subdirectories. CLI defaults to --model-type tiered. Binary size increased from ~12MB to ~21MB due to tiered model (34 models vs 1). All 187 tests pass. Embedded tiered inference verified: email, IP, date, coordinates all classified correctly without --model-type flag."
<!-- SECTION:FINAL_SUMMARY:END -->
