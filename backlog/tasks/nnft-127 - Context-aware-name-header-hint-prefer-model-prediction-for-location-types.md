---
id: NNFT-127
title: 'Context-aware name header hint: prefer model prediction for location types'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-25 03:38'
updated_date: '2026-02-25 06:31'
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
- [x] #1 world_cities.name correctly predicted as city (not full_name)
- [ ] #2 countries.name correctly predicted as country (not full_name)
- [x] #3 Existing full_name predictions not regressed (titanic Name, people_directory name)
- [x] #4 Profile eval >=70/74
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
countries.name remains a miss — model splits 51% identity vs 47% geography for country names. No clean signal to prefer country over city/region/full_name from just a "name" header. Intractable without cross-column context.

world_cities.name fixed via geography protection Case 2 (attractor-demoted prediction rescued by geography votes). Achieves +1 from this task.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added geography protection to header hint override logic in classify_column_with_header(). When Model2Vec hints full_name for a "name" column header, two new checks prevent overriding correct geography predictions:

1. **Case 1 (location keep)**: If the model already predicts a location type (city, country, region, state, continent), keep it instead of overriding to full_name.
2. **Case 2 (location rescue)**: If the prediction was attractor-demoted to generic but geography votes exist (>=10%), pick the top geography type.

Result: world_cities.name now correctly predicts geography.location.city (+1 eval). countries.name remains full_name — the model splits 51% identity vs 47% geography for country names, making this intractable without cross-column context.

Tests: 294 pass, taxonomy 169/169, eval 70/74 (combined with NNFT-128).
File: crates/finetype-model/src/column.rs (lines 284-334)
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
