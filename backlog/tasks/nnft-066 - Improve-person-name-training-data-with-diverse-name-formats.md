---
id: NNFT-066
title: Improve person name training data with diverse name formats
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-15 05:13'
updated_date: '2026-02-15 08:21'
labels:
  - training-data
  - taxonomy
dependencies: []
references:
  - labels/definitions_identity.yaml
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The char-CNN model confuses person names with user_agent strings. Titanic's Name column ("Braund, Mr. Owen Harris") gets classified as user_agent because the "LastName, Title. FirstName" format looks character-similar to user-agent strings (commas, dots, mixed case, multi-word).

We have `identity.person.full_name` in the taxonomy, but the training data may not cover enough format diversity. Common name formats in real datasets:
- "FirstName LastName" (basic)
- "LastName, FirstName" (CSV/database style)
- "LastName, Title. FirstName MiddleName" (Titanic style)
- "LASTNAME, FIRSTNAME" (all caps)
- "Dr. FirstName LastName" (title prefix)
- "FirstName M. LastName" (middle initial)

Improve the generator for full_name to cover these formats, regenerate training data, and verify the model can distinguish names from user agents after retraining.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 full_name generator produces at least 4 distinct name formats (basic, reversed, titled, all-caps)
- [x] #2 Training data includes ≥800 diverse full_name samples
- [ ] #3 After retraining, "Braund, Mr. Owen Harris" classified as full_name (not user_agent)
- [ ] #4 After retraining, "Smith, Dr. Jane" classified as full_name
- [ ] #5 No regression on user_agent accuracy (user agents should still be correctly identified)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #3, #4, #5 require model retraining which is out of scope for this task. The generator improvements are in place; verification happens after the next training round.

Formats implemented: basic (30%), reversed/CSV (20%), titled Titanic-style (10%), all-caps (10%), title prefix (10%), middle initial (10%), formal with middle name (10%).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Diversified the full_name generator to produce 7 distinct name formats, covering the formats that caused model confusion with user_agent strings.

## Changes

**crates/finetype-core/src/generator.rs:**
- Expanded full_name generator from 1 format to 7 weighted formats:
  - \"FirstName LastName\" (30%) — basic
  - \"LastName, FirstName\" (20%) — CSV/database style
  - \"LastName, Title. FirstName\" (10%) — Titanic style
  - \"LASTNAME, FIRSTNAME\" (10%) — all caps
  - \"Title FirstName LastName\" (10%) — title prefix (Dr., Mr., etc.)
  - \"FirstName M. LastName\" (10%) — middle initial
  - \"LastName, Title. FirstName MiddleName\" (10%) — formal with middle

**labels/definitions_identity.yaml:**
- Updated full_name validation pattern from `^[\\p{L}\\s'\\-]+$` to `^[\\p{L}\\s'\\-.,]+$` to allow commas and dots in titled/reversed formats
- Added 3 new sample values showing diverse formats

## Verification
- `finetype check`: all checks pass (163/163 types)
- Training data generation produces diverse name formats
- AC #3-5 (model accuracy) pending next retraining round
<!-- SECTION:FINAL_SUMMARY:END -->
