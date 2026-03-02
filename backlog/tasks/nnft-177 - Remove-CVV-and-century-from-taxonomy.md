---
id: NNFT-177
title: Remove CVV and century from taxonomy
status: Done
assignee: []
created_date: '2026-03-02 05:50'
updated_date: '2026-03-02 06:23'
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
- [x] #1 identity.payment.cvv removed from labels/definitions_identity.yaml
- [x] #2 datetime.component.century removed from labels/definitions_datetime.yaml
- [x] #3 Generators removed or updated for both types
- [x] #4 cargo run -- check passes (taxonomy/generator alignment)
- [x] #5 cargo test passes with no regressions
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Removed identity.payment.cvv (false-positive magnet, security concern) and datetime.component.century (Roman numerals only, no transformation contract) from taxonomy.

Changes:
- Removed both types from YAML definitions
- Removed generators for both types
- cargo run -- check: 166/166 passing, 8300/8300 samples
- cargo test: 357/357 passing

Note: ACs #6-8 (training data, model retrain, eval baselines) deferred to NNFT-181.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
