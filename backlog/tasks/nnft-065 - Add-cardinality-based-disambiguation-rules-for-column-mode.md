---
id: NNFT-065
title: Add cardinality-based disambiguation rules for column mode
status: To Do
assignee: []
created_date: '2026-02-15 05:13'
labels:
  - feature
  - disambiguation
dependencies:
  - NNFT-063
references:
  - crates/finetype-model/src/column.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extend column-mode disambiguation in `column.rs` with rules based on value distribution characteristics beyond just numeric range analysis.

Current disambiguation only handles numeric types (port, postal_code, year, increment, street_number). String-type columns have no disambiguation at all, leading to issues like:
- Embarked (S/C/Q) → boolean (should be categorical)
- SibSp (0-8) → boolean (should be integer or ordinal)
- Sex (male/female) → word (should be gender or categorical)

New rules to add:
- **Low cardinality detection**: If unique values ≤ 2 on numeric data → boolean candidate. If 3-20 unique values on string data → categorical candidate.
- **Small integer spread**: If column has integer values spanning 0-N where N > 1, with more than 2 unique values → NOT boolean (integer_number or ordinal)
- **Single-character columns**: If all values are single characters → categorical (not boolean)
- **Gender detection**: If all values are in {"male","female","m","f","M","F","Male","Female"} → identity.person.gender

Depends on NNFT-063 (categorical/ordinal types must exist in taxonomy first).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Low cardinality rule: 3-20 unique string values → categorical candidate
- [ ] #2 Small integer spread rule: 0-N with N>1 and >2 uniques → not boolean
- [ ] #3 Single-character column rule: all single chars → categorical (not boolean)
- [ ] #4 Gender detection rule: known gender value set → identity.person.gender
- [ ] #5 Titanic SibSp and Parch no longer classified as boolean
- [ ] #6 Titanic Embarked no longer classified as boolean
- [ ] #7 Titanic Sex classified as gender (or categorical as fallback)
- [ ] #8 Unit tests for each new cardinality rule
<!-- AC:END -->
