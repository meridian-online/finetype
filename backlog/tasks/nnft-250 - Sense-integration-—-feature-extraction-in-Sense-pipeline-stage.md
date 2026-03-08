---
id: NNFT-250
title: Sense integration — feature extraction in Sense pipeline stage
status: To Do
assignee: []
created_date: '2026-03-07 23:56'
labels:
  - model
  - pipeline
milestone: m-12
dependencies:
  - NNFT-247
  - NNFT-248
  - NNFT-249
references:
  - crates/finetype-model/src/sense.rs
  - crates/finetype-model/src/column.rs
  - crates/finetype-model/src/label_category_map.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Integrate feature extraction into the Sense→Sharpen pipeline so features flow naturally through inference:

1. **Per-value features:** Feature extractor runs on each sampled value, features passed to augmented CharCNN during batch classification (Sharpen stage)
2. **Aggregated column-level features:** Mean/std/mode of per-value features computed and made available to the disambiguation stage for column-level decisions
3. **Deterministic pre-filter:** Leading-zero detection and other high-signal features can short-circuit disambiguation for known-confusing pairs (numeric_code/postal_code, cpt/postal_code)

Feature extraction runs during Sense stage so results are available before CharCNN batch — adds <5ms overhead per column.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Feature extraction runs during Sense stage on sampled values
- [ ] #2 Per-value features passed to augmented CharCNN in Sharpen stage batch
- [ ] #3 Aggregated column features (mean/std/mode) available to disambiguation rules
- [ ] #4 Leading-zero pre-filter resolves numeric_code vs postal_code without model
- [ ] #5 Inference latency increase <5ms per column vs current pipeline
- [ ] #6 Existing Sense→Sharpen pipeline flow preserved — no breaking changes
- [ ] #7 Profile eval accuracy improves over 74.1% label baseline on 250-type taxonomy
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
