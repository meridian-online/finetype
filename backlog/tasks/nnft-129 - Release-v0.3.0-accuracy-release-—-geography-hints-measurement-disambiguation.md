---
id: NNFT-129
title: 'Release v0.3.0: accuracy release — geography hints, measurement disambiguation'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-25 06:31'
updated_date: '2026-02-25 06:33'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
v0.3.0 accuracy release with two targeted disambiguation improvements: geography-aware header hints (NNFT-127) and measurement type disambiguation (NNFT-128). Profile eval 68/74 → 70/74 (94.6%).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version bumped to 0.3.0
- [x] #2 CHANGELOG updated
- [x] #3 CLAUDE.md updated
- [x] #4 Tests pass (294)
- [x] #5 Taxonomy check passes (169/169)
- [x] #6 Profile eval 70/74
- [x] #7 Tagged and pushed
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released v0.3.0 with two accuracy improvements:

1. Geography-aware header hints (NNFT-127): prevents full_name hint from overriding correct location predictions. Two cases — keeps model prediction when already a location type, rescues attractor-demoted predictions when geography votes exist. Fixes world_cities.name → city.

2. Measurement disambiguation (NNFT-128): when both hint and prediction are measurement types (age/height/weight), trusts the header since values are numerically indistinguishable. Fixes medical_records.height_in → height.

Profile eval: 68/74 → 70/74 (94.6%). No regressions. 294 tests pass, taxonomy 169/169 aligned.

Tag: v0.3.0, pushed to origin/main.
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
