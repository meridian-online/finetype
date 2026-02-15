---
id: NNFT-066
title: Improve person name training data with diverse name formats
status: To Do
assignee: []
created_date: '2026-02-15 05:13'
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
- [ ] #1 full_name generator produces at least 4 distinct name formats (basic, reversed, titled, all-caps)
- [ ] #2 Training data includes ≥800 diverse full_name samples
- [ ] #3 After retraining, "Braund, Mr. Owen Harris" classified as full_name (not user_agent)
- [ ] #4 After retraining, "Smith, Dr. Jane" classified as full_name
- [ ] #5 No regression on user_agent accuracy (user agents should still be correctly identified)
<!-- AC:END -->
