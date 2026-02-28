---
id: NNFT-169
title: Create LabelCategoryMap for Sense output masking
status: To Do
assignee: []
created_date: '2026-02-28 23:06'
labels:
  - sense-sharpen
  - feature
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement LabelCategoryMap that maps all 163 FineType type labels to their primary Sense BroadCategory, plus overlap types eligible in secondary categories. Used by ColumnClassifier for masked vote aggregation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 New label_category_map.rs module
- [ ] #2 All 163 types assigned to exactly one primary BroadCategory
- [ ] #3 6 overlap types have also_eligible secondary categories
- [ ] #4 category_for() returns primary category
- [ ] #5 is_eligible() checks primary + also_eligible
- [ ] #6 eligible_labels() returns all labels for a category
- [ ] #7 Unit test verifies total = 163 and no duplicates
- [ ] #8 Exported from lib.rs
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
