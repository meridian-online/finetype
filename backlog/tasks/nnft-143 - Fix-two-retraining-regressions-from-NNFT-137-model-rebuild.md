---
id: NNFT-143
title: Fix two retraining regressions from NNFT-137 model rebuild
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-26 11:04'
updated_date: '2026-02-26 11:39'
labels:
  - accuracy
  - disambiguation
dependencies:
  - NNFT-137
references:
  - crates/finetype-model/src/column.rs
  - eval/eval_profile.sql
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
NNFT-137 retrained the full tiered-v2 model with 1000 samples × 10 epochs to include entity_name and paragraph types. Two columns that were correct at the 70/74 baseline now regress:

1. **world_cities.name** — predicts `identity.person.full_name` instead of `geography.location.city` (confidence 0.60, tagged `[header_hint:name]`). The "name" header triggers a full_name hint, and geography protection doesn't fire because the retrained T1 VARCHAR model routes city names to the `person` category instead of `location`. The geography signal is lost before the hint guard can check for it.

2. **datetime_formats.utc_offset** — predicts `datetime.time.hm_24h` instead of `datetime.offset.utc` (confidence 0.325). UTC offsets like "+05:30" and "-08:00" are structurally similar to 24h times like "14:30". The retrained T2 model shifted its decision boundary.

Both are model boundary shifts from complete retraining, not caused by the entity_name/paragraph taxonomy additions. Potential fix approaches:
- Disambiguation rule for UTC offsets (leading +/- distinguishes from time)
- Strengthen geography protection for location columns when header is "name"
- Targeted training data improvements for the affected T2 models
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 world_cities.name correctly predicts geography.location.city
- [x] #2 datetime_formats.utc_offset correctly predicts datetime.offset.utc
- [x] #3 Profile eval restores to ≥70/74 (no net regressions from v0.3.0 baseline)
- [ ] #4 No regressions on other columns
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Regression 1: UTC offset → hm_24h (FIXABLE — disambiguation rule)

Root cause: Model classifies "+05:30", "-08:00" as time types (hm_24h 40%, hms_24h 40%)
because they look like HH:MM times. But UTC offsets always start with + or -.

Fix: Add UTC offset override rule (Rule 17) in column.rs:
- After majority vote, if top vote is any datetime.time.* type
- Check if ≥80% of non-empty values match pattern ^[+-]\d{2}:\d{2}$
- If so, override to datetime.offset.utc
- Run BEFORE attractor demotion (similar position to duration override)

This is a clean syntactic distinction — no false positives possible.

### Regression 2: world_cities.name → full_name (NOT FIXABLE without retraining)

Root cause: The retrained T1 VARCHAR model routes city names to person/text
categories instead of location. Vote distribution has 0% geography types.
Geography protection cannot rescue what the model does not produce.

The entity_name addition absorbed some city-name predictions. City names
("Tokyo", "London") look like single proper nouns — indistinguishable from
brand names at the character level. No disambiguation rule can help because
there are no geography votes to rescue.

This requires model training improvements (better city generator diversity)
or entity_name generator tightening (require business suffixes/numbers).
Scope for a separate task.

### Expected outcome
- Fix UTC offset → 70/74 (matches v0.3.0 baseline)
- world_cities.name stays wrong → follow-up task for model training
- Update AC #1 to acknowledge this needs model retraining
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
UTC offset disambiguation rule (Rule 17) implemented and verified.
- Added disambiguate_utc_offset_override() function: checks for [+-]HH:MM pattern
- Triggers when top vote is datetime.time.* or rfc_3339, and ≥80% of values match
- 5 unit tests added and passing
- Profile eval restored to 70/74 (94.6% label, 97.3% domain)
- datetime_formats.utc_offset now correctly predicts datetime.offset.utc

City generator expansion completed:
- EN_US: 16 → 41 cities (added San Jose, Fort Worth, Las Vegas, El Paso, New Orleans, Salt Lake City, etc.)
- EN_GB: 12 → 20 cities (added Newcastle upon Tyne, Stoke-on-Trent, etc.)
- EN_AU: 10 → 16 cities (added Sunshine Coast, Alice Springs, Mount Gambier, etc.)
- EN_CA: 10 → 16 cities (added Saint John, Thunder Bay, Fort McMurray, etc.)
- EN: 10 → 27 cities (added Kuala Lumpur, Buenos Aires, Rio de Janeiro, Dar es Salaam, etc.)
- FR: 16 → 25 cities (added Saint-Étienne, Aix-en-Provence, Le Havre, etc.)
- ES: 16 → 20 cities (added San Sebastián, Las Palmas de Gran Canaria, etc.)
- New locales: PT_BR (16 cities), HI (12 cities), TR (12 cities)
- Updated geography.location.city locales list to include PT_BR, HI, TR
- Training data generated: 368,000 samples with 4-level locale labels
- Overnight training kicked off: 1000 samples × 10 epochs, tiered-v2 model
<!-- SECTION:NOTES:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
