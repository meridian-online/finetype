---
id: NNFT-090
title: Address top profile eval misclassification patterns
status: To Do
assignee: []
created_date: '2026-02-17 22:44'
labels:
  - accuracy
  - disambiguation
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The profile eval with tiered-v2 still has several systematic misclassification patterns that could be addressed with targeted disambiguation rules or training improvements:

Top losses (format-detectable + partial):
- number → street_number (4 cols): Generic integer columns misclassified as street numbers
- code → cvv (4 cols): Alphanumeric codes confused with CVV payment codes
- number → postal_code (3 cols): Integer columns confused with postal codes
- weight → decimal_number (2 cols): Weight measurements not recognized
- code → issn (2 cols): Generic codes confused with ISSN
- region → continent (2 cols): Region strings misclassified as continents
- boolean → terms (2 cols): Binary booleans predicted as boolean terms
- file format → url (2 cols): MIME types confused with URLs

These patterns suggest opportunities for column-mode disambiguation rules based on value distributions, header names, or format validation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Profile eval format-detectable label accuracy improves beyond 72.6%
- [ ] #2 At least 3 of the top misclassification patterns addressed
- [ ] #3 No regression on existing correct classifications
- [ ] #4 New disambiguation rules have unit tests
<!-- AC:END -->
