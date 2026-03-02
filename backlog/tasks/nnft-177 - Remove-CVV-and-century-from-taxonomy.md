---
id: NNFT-177
title: Remove CVV and century from taxonomy
status: To Do
assignee: []
created_date: '2026-03-02 05:50'
labels:
  - taxonomy
  - v0.5.1
dependencies: []
references:
  - discovery/taxonomy-revision/BRIEF.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 1 of taxonomy revision (v0.5.1): remove two low-value types identified in NNFT-176 discovery.

- Remove `identity.payment.cvv` — 3-4 digit integers, extremely high false-positive rate, low analyst value, security concern (CVVs should never appear in analytical datasets)
- Remove `datetime.component.century` — detects Roman numerals (XIX, XX, XXI) only, no format_string, no DuckDB transformation contract, vanishingly rare as standalone column

This is the simplest taxonomy change and should be done first to establish the v0.5.1 baseline.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 identity.payment.cvv removed from labels/definitions_identity.yaml
- [ ] #2 datetime.component.century removed from labels/definitions_datetime.yaml
- [ ] #3 Generators removed or updated for both types
- [ ] #4 cargo run -- check passes (taxonomy/generator alignment)
- [ ] #5 cargo test passes with no regressions
- [ ] #6 Training data regenerated without removed types
- [ ] #7 Model retrained on updated taxonomy
- [ ] #8 Eval baselines updated (if either type appeared in eval datasets)
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
