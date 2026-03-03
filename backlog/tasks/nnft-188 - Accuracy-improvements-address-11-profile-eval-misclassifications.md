---
id: NNFT-188
title: 'Accuracy improvements: address 11 profile eval misclassifications'
status: To Do
assignee: []
created_date: '2026-03-03 03:49'
labels:
  - accuracy
  - model
dependencies:
  - NNFT-181
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Profile eval is at 108/119 (90.8% label, 96.6% domain) after v0.5.1 model retrain. There are 11 remaining misclassifications that should be investigated and fixed to improve accuracy toward the v0.5.0 baseline of 96.7%.

The misclassifications fall into distinct categories:
1. **Numeric confusion** (5 misses): iris decimal columns → percentage (×4), pressure_atm → latitude
2. **Entity/location confusion** (3 misses): countries.name/world_cities.name → full_name (×2), covid Country → city
3. **Format confusion** (2 misses): airports.timezone → iso_microseconds, books_catalog.publisher → gender
4. **Categorical confusion** (1 miss): people_directory.job_title → entity_name instead of categorical

The timezone misclassification also causes an actionability regression (98% → 27%) because the eval tries to parse timezone strings with ISO microsecond format strings.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Profile eval label accuracy ≥ 112/119 (94%)
- [ ] #2 Actionability score restored to ≥ 95% for datetime columns
- [ ] #3 No new regressions introduced (existing correct predictions preserved)
- [ ] #4 Eval report generated with updated baselines
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
