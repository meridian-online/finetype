---
id: NNFT-127
title: 'Context-aware name header hint: prefer model prediction for location types'
status: To Do
assignee: []
created_date: '2026-02-25 03:38'
labels:
  - accuracy
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The name header hint maps unconditionally to full_name, overriding correct model predictions. This causes 2 eval misses:
- world_cities.name: model correctly predicts city, but name hint overrides to full_name
- countries.name: model correctly predicts country, but name hint overrides to full_name

Fix: When header hint returns full_name but the model top vote is a location type (city, country, region) with reasonable vote share (>=30%), prefer the model prediction over the hint. The model already got it right — the hint is actively harming.

File: crates/finetype-model/src/column.rs — header hint override logic in apply_header_hint()
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 world_cities.name correctly predicted as city (not full_name)
- [ ] #2 countries.name correctly predicted as country (not full_name)
- [ ] #3 Existing full_name predictions not regressed (titanic Name, people_directory name)
- [ ] #4 Profile eval >=70/74
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
