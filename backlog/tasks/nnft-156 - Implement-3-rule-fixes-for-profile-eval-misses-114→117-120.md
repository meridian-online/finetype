---
id: NNFT-156
title: Implement 3 rule fixes for profile eval misses (114→117/120)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-27 21:50'
updated_date: '2026-02-27 22:14'
labels:
  - accuracy
  - disambiguation
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Three remaining profile eval misses are rule-fixable in column.rs. Fix 1: CVV postal_code overcall — make numeric_postal_code_detection yield to header hints. Fix 2: world_cities.name last_name overcall — expand geography protection to cover all person-name hints. Fix 3: countries.name entity_name overcall — add geography rescue during entity demotion.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Fix 1: numeric_postal_code_detection made generic in is_generic_prediction()
- [x] #2 Fix 2: Geography protection expanded to PERSON_NAME_HINTS array
- [x] #3 LOCATION_TYPES extracted to module-level const
- [x] #4 Unit tests for all 3 fixes
- [x] #5 cargo test passes
- [x] #6 cargo clippy -D warnings clean
- [x] #7 make eval-report shows 116/120 label accuracy (96.7%)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Extract LOCATION_TYPES to module-level const (prerequisite for Fix 3)
2. Fix 1: Add numeric_postal_code_detection to is_generic_prediction()
3. Fix 2: Expand geography protection from single full_name to PERSON_NAME_HINTS array
4. Fix 3: Add geography rescue to entity demotion (check vote_distribution for location types)
5. Add unit tests for all 3 fixes
6. Run cargo test + cargo clippy
7. Run make eval-report and verify 117/120
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fix 3 (entity demotion geography rescue) reverted — caused 3 regressions. Entity demotion produces entity_name which counts as correct via entity_name↔full_name eval interchangeability for GT="name" columns. Geography rescue changed entity_name to specific geography types (city/region), breaking interchangeability. For countries.name (GT="country"), the model votes city>country so rescue picks the wrong type anyway. Countries.name needs entity classifier class probabilities (place vs org), not vote_distribution — deferred to follow-up task.

Final scope: Fix 1 (CVV) + Fix 2 (geography protection) = +2 points (114→116).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed 2 profile eval misclassifications, improving label accuracy from 114/120 (95.0%) to 116/120 (96.7%) and domain accuracy from 116/120 (96.7%) to 118/120 (98.3%). Zero regressions.

Changes (all in crates/finetype-model/src/column.rs):

- Fix 1 (CVV): Added numeric_postal_code_detection as Signal 1b in is_generic_prediction(). The numeric postal code heuristic is pattern-based, not model-driven, and should yield to explicit header hints. Fixes codes_and_ids.cvv: postal_code(0.80) → cvv via header hint override.

- Fix 2 (geography protection): Expanded geography protection guard from single full_name check to PERSON_NAME_HINTS array (full_name, last_name, first_name). Model2Vec returns last_name for "name" headers on city/country columns, bypassing the old full_name-only guard. Fixes world_cities.name: last_name(0.60) → city via geography protection.

- Extracted LOCATION_TYPES to module-level const for reuse across geography protection and future entity demotion improvements.

- Added 3 unit tests: numeric_postal_code_detection genericity, PERSON_NAME_HINTS coverage, LOCATION_TYPES module-level extraction.

Descoped: Fix 3 (entity demotion geography rescue) was implemented and reverted. The geography rescue checked vote_distribution for location types during entity demotion, intending to rescue countries.name from entity_name to country. Caused 3 regressions because entity_name↔full_name eval interchangeability counts entity_name as correct for GT="name" columns (airports.name, multilingual.name, sports_events.venue), but specific geography types break interchangeability. Additionally, for countries.name the model votes city>country, so the rescue picks the wrong type.

Tests: cargo test (319 pass), cargo clippy clean, make eval-report (116/120 label, 118/120 domain).
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
