---
id: NNFT-169
title: Create LabelCategoryMap for Sense output masking
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-28 23:06'
updated_date: '2026-03-01 00:15'
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
- [x] #1 New label_category_map.rs module
- [x] #2 All 163 types assigned to exactly one primary BroadCategory
- [x] #3 6 overlap types have also_eligible secondary categories
- [x] #4 category_for() returns primary category
- [x] #5 is_eligible() checks primary + also_eligible
- [x] #6 eligible_labels() returns all labels for a category
- [x] #7 Unit test verifies total = 163 and no duplicates
- [x] #8 Exported from lib.rs
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Derive exact 163-type mapping from design doc + training script reconciliation
2. Create label_category_map.rs with static arrays per category
3. Implement LabelCategoryMap struct with category_for(), is_eligible(), eligible_labels()
4. Add also_eligible entries for overlap types
5. Unit test verifying total=163, no duplicates, overlap correctness
6. Export from lib.rs
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented LabelCategoryMap (label_category_map.rs) mapping all 163 FineType type labels to 6 Sense BroadCategory values for output masking.

Mapping reconciles design doc type-specific assignments with training script domain-based heuristic:
- temporal (46): all datetime.* types
- numeric (14): measurements, small integers, numeric representations
- geographic (16): all geography.* types
- entity (9): person names + entity_name
- format (48): structured identifiers, containers, codes, sequences
- text (30): free text, low-cardinality enums, categorical values

8 ALSO_ELIGIBLE overlap entries ensure types at category boundaries pass the mask in either direction: postal_code/calling_code (geographic↔format), coordinates/lat/lng (geographic↔numeric), email/phone (format↔entity), credit_card_network (text↔format).

LabelCategoryMap struct provides: category_for() for primary lookup, is_eligible() for mask checking (primary + secondary), eligible_labels() for all labels in a category. Verified 100% taxonomy alignment (163/163 match).

Tests: 8 unit tests — total count, per-category counts, no duplicates, primary lookups, overlap handling, eligible_labels counts, sorted arrays. Full suite: 245 pass.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
