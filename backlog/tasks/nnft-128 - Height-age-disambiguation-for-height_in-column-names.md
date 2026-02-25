---
id: NNFT-128
title: Height/age disambiguation for height_in column names
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-25 03:38'
updated_date: '2026-02-25 06:34'
labels:
  - accuracy
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
medical_records.height_in is predicted as age at 0.967 confidence. The height header hint exists but does not match height_in (the _in suffix for inches). Model2Vec semantic matching should handle this — verify height_in similarity score. If below threshold, add height_in as a synonym in prepare_model2vec.py header hints.

File: crates/finetype-model/src/column.rs or scripts/prepare_model2vec.py
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 height_in correctly predicted as height (not age)
- [x] #2 No regression on other height/age predictions
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Identify root cause: hardcoded header hint already maps height_in → height (via contains("height")), but override fails because age confidence is 0.967 and height not in vote distribution
2. Add measurement disambiguation: when hint and prediction are both in {age, height, weight}, trust the header (values are numerically indistinguishable)
3. Verify no regressions on other age/height/weight columns
4. Run eval-profile to confirm +1
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fix implemented: added MEASUREMENT_TYPES disambiguation block in classify_column_with_header(). When hint and model prediction are both measurement types (age/height/weight), trust the header since values are numerically indistinguishable.

Verified: only medical_records.height_in affected. titanic.Age, people_directory.age, people_directory.height_cm, people_directory.weight_kg, medical_records.weight_lbs all unchanged (model already matches hint).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added measurement disambiguation to header hint override logic. Age, height, and weight values are numerically indistinguishable (all small integers in overlapping ranges), so the model cannot differentiate them from values alone. When the header provides a specific measurement hint (e.g., "height_in" → height) but the model predicts a different measurement type (e.g., age at 0.967), the header now wins.

The MEASUREMENT_TYPES const ({age, height, weight}) defines the disambiguation group. This is a targeted fix — only triggers when both hint and prediction are in this group.

Result: medical_records.height_in now correctly predicts identity.person.height (+1 eval).
No regressions: all other age/height/weight predictions already matched their hints.

Tests: 294 pass, taxonomy 169/169, eval 70/74.
File: crates/finetype-model/src/column.rs (lines 255-270)
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
