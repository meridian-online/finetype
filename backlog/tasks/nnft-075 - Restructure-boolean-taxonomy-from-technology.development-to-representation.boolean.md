---
id: NNFT-075
title: >-
  Restructure boolean taxonomy from technology.development to
  representation.boolean
status: To Do
assignee: []
created_date: '2026-02-16 03:22'
labels:
  - taxonomy
  - model
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Move boolean classification from technology.development.boolean to the representation domain with format-specific subtypes. Booleans are a data representation concept, not a technology/development one. Splitting by string format enables better casting and normalization.

Current: technology.development.boolean (single catch-all)

Proposed:
- representation.boolean.binary — 0/1 values
- representation.boolean.initials — T/F, Y/N (single character)
- representation.boolean.terms — True/False, Yes/No, On/Off, Enabled/Disabled

This is a breaking taxonomy change requiring model retraining.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove technology.development.boolean from taxonomy
- [ ] #2 Add representation.boolean.binary with generator for 0/1 values
- [ ] #3 Add representation.boolean.initials with generator for T/F, Y/N variants
- [ ] #4 Add representation.boolean.terms with generator for True/False, Yes/No, On/Off variants
- [ ] #5 Generators produce case variants (TRUE/true/True, T/t, etc.)
- [ ] #6 Column classifier rules updated for boolean subtypes
- [ ] #7 finetype_cast normalization handles all three boolean formats
- [ ] #8 DuckDB type mapping: all three map to BOOLEAN
- [ ] #9 Model retrained with new labels
- [ ] #10 Existing tests updated for new labels
<!-- AC:END -->
