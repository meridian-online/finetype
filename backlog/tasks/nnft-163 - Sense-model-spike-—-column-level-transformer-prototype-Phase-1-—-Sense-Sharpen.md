---
id: NNFT-163
title: >-
  Sense model spike — column-level transformer prototype (Phase 1 — Sense &
  Sharpen)
status: To Do
assignee:
  - '@nightingale'
created_date: '2026-02-28 03:41'
labels:
  - architecture
  - ml
  - sense-and-sharpen
  - spike
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Time-boxed 1-week spike to validate the core hypothesis: a column-level transformer can outperform the current Sense-equivalent (entity classifier + semantic hints + disambiguation rules) on semantic classification.

Two target outputs:
1. Broad category routing (format / entity / numeric / temporal / text / geographic) — replaces T0→T1 routing
2. Entity subtype (person / place / organisation / creative_work) — replaces standalone entity classifier

Column name is an input feature to the model (not a separate post-classification system). This is the single biggest architectural improvement: header signal and model prediction become a unified decision, collapsing header hint overrides, geography protection, and entity demotion guard complexity.

Sample 50 values per column using stratified sampling. Test 20 vs 50 during the spike.

Test two architectures:
- A. Lightweight attention over Model2Vec value embeddings (fast, minimal)
- B. Small transformer encoder over character sequences (powerful, slower)

Train on SOTAB columns (2,911 entity + format-type columns) + profile eval datasets.

This is Phase 1 of the Sense & Sharpen pivot (decision-004). Phase 0 (taxonomy audit) runs in parallel.

Go criteria:
- Broad category accuracy > 95%
- Entity subtype accuracy > 78% (exceeds current 75.8%)
- Column inference < 50ms for 50 sampled values
- Clear path to Candle/Rust implementation

Defer to Phase 2+:
- Locale signal detection (requires training data curation)
- Confidence calibration (training hyperparameter, not spike priority)
- Rust/Candle implementation (depends on spike findings)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Curate training data: extract column-level (sampled_values, broad_category, entity_subtype) from SOTAB
- [ ] #2 Map Schema.org annotations → broad category labels (format/entity/numeric/temporal/text/geographic)
- [ ] #3 Implement Architecture A: lightweight attention over Model2Vec value embeddings with column name input
- [ ] #4 Implement Architecture B: small transformer encoder with column name input
- [ ] #5 Train both architectures on SOTAB + profile eval data
- [ ] #6 Evaluate broad category routing accuracy (target: >95%)
- [ ] #7 Evaluate entity subtype accuracy (target: >78%, baseline 75.8%)
- [ ] #8 Benchmark column-level inference speed at 20 and 50 sampled values (target: <50ms for 50 values)
- [ ] #9 Compare against current system on identical columns
- [ ] #10 Produce FINDING.md with go/no-go recommendation, architecture comparison, and speed benchmarks
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
