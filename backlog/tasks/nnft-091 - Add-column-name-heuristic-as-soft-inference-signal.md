---
id: NNFT-091
title: Add column-name heuristic as soft inference signal
status: Done
assignee: []
created_date: '2026-02-17 22:44'
updated_date: '2026-02-17 22:45'
labels:
  - accuracy
  - disambiguation
  - feature
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
SUPERSEDED by NNFT-067 (already implemented). Header hints already exist with 27+ patterns, confidence-based override logic, and --no-header-hint flag. If further improvements are needed, they should be scoped as refinements to the existing system, not a new feature.

Consider folding any remaining header-hint improvements into NNFT-090 (misclassification patterns)."
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Column name matching integrated into ColumnClassifier disambiguation pipeline
- [ ] #2 Header-based boost improves partially-detectable type accuracy
- [ ] #3 Heuristic is soft — doesn't override high-confidence model predictions
- [ ] #4 Mapping covers at least 20 common column name patterns (weight, height, latitude, longitude, city, country, etc.)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Superseded by NNFT-067 which already implements column-name header hints with 27+ patterns and confidence-based override logic."
<!-- SECTION:FINAL_SUMMARY:END -->
