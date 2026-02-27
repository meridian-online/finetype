---
id: NNFT-145
title: >-
  Investigation: Disambiguate city vs entity_name using validation and enum
  patterns
status: To Do
assignee:
  - '@nightingale'
created_date: '2026-02-27 00:34'
labels:
  - discovery
  - disambiguation
  - entity_name
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
world_cities.name is misclassified as full_name/entity_name after the NNFT-137 retrain. Brute-force generator expansion failed — it over-corrected the T1 router and caused 4 new regressions (67/74).

The root cause is that city names and person/entity names are indistinguishable at the character level ("London" vs "Johnson"). Throwing more training data at this is a zero-sum game at the T1 router.

Instead, investigate smarter disambiguation using validation and enum patterns:
- City names are a closed(ish) set — could we validate against a city name enum (like we do for country codes, calling codes, CLDR data)?
- What data sources exist for city name lists? GeoNames, OpenStreetMap, UN LOCODE?
- Could a validation_by_locale approach work for cities like it does for postal_code and phone_number?
- What about a disambiguation rule that checks if values appear in a city lookup vs the entity_name/full_name pattern?
- How would this interact with the existing header hint geography protection (which already works when the model has geography votes)?

This is about finding a surgical approach that works within the existing disambiguation framework, not retraining the model.

Time-box: 2-4 hours. Output: finding with recommended approach.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Analysis of viable city name data sources (coverage, licensing, size)
- [ ] #2 Prototype or feasibility assessment of enum-based city validation
- [ ] #3 Recommendation: specific disambiguation approach with trade-offs
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
