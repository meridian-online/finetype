---
id: NNFT-170
title: Integrate Sense into ColumnClassifier pipeline
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-28 23:06'
updated_date: '2026-03-01 00:26'
labels:
  - sense-sharpen
  - feature
dependencies:
  - NNFT-168
  - NNFT-169
  - NNFT-166
  - NNFT-167
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Wire SenseClassifier + LabelCategoryMap into ColumnClassifier. Add classify_sense_sharpen() method that: (1) encodes values+header with Model2Vec, (2) runs Sense for broad category, (3) masks CharCNN votes to category, (4) applies scoped disambiguation rules, (5) handles entity subtype demotion. Falls back to current pipeline when sense is None.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ColumnClassifier gains sense, model2vec, label_map optional fields
- [x] #2 classify_sense_sharpen() implements the full Sense→Sharpen pipeline
- [x] #3 Masked vote aggregation with fallback to unmasked when all votes filtered
- [x] #4 Entity demotion via Sense replaces EntityClassifier Rule 18
- [x] #5 Header hints bypassed when Sense is active
- [x] #6 When sense is None, classify_column and classify_column_with_header unchanged
- [x] #7 All existing column.rs tests pass unchanged (fallback path)
- [x] #8 New tests for Sense pipeline path
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add optional fields to ColumnClassifier: sense (Option<SenseClassifier>), model2vec (Option<Model2VecResources>), label_map (Option<LabelCategoryMap>)
2. Add setter methods: set_sense(), set_model2vec(), set_label_map()
3. Implement classify_sense_sharpen() private method with Sense→Sharpen pipeline:
   a. Encode header+values with Model2Vec (first 50 values)
   b. Run SenseClassifier::classify() for broad category + entity subtype
   c. Run CharCNN batch on all sampled values (reuses existing infrastructure)
   d. Remap collapsed labels
   e. Masked vote aggregation: filter to eligible labels per LabelCategoryMap
   f. Fallback to unmasked when all votes filtered
   g. Apply disambiguation rules (scoped to category)
   h. Entity handling via Sense entity_subtype (replaces Rule 18)
   i. Post-hoc locale detection (unchanged)
4. Modify classify_column_with_header(): when sense is Some, call classify_sense_sharpen() instead of classify_column() + header hints
5. Keep classify_column() unchanged (AC #6 backward compat)
6. Add unit tests for Sense pipeline path
7. Verify all existing tests pass unchanged
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Integrated the Sense→Sharpen pipeline into ColumnClassifier (column.rs), enabling broad semantic category prediction to mask CharCNN votes.

Changes:
- Added `sense`, `model2vec`, `label_map` optional fields to ColumnClassifier struct
- Added `set_sense()` method that accepts all three resources together
- Added `has_sense()` method to check if Sense pipeline is active
- `classify_column_with_header()` now dispatches to `classify_sense_sharpen()` when Sense is active, falling back to the legacy header-hint path when absent
- `classify_sense_sharpen()` implements the full pipeline: sample → M2V encode (header + 50 values) → Sense classify → CharCNN batch → remap → masked vote aggregation → disambiguation rules → entity demotion via Sense subtype → locale detection
- Masked vote aggregation filters CharCNN votes to category-eligible labels per LabelCategoryMap. Safety valve: falls back to unmasked when all votes filtered
- Entity demotion via Sense entity subtype (Person/Place/Organization/CreativeWork) replaces Rule 18 + EntityClassifier when Sense is active
- Header hints, geography protection, measurement disambiguation, and entity demotion guard are all subsumed by Sense (header is a Sense input)
- `classify_column()` (no header) is unchanged — full backward compatibility

Tests:
- 7 new Sense pipeline tests: field defaults, set_sense enables, fallback without sense, unanimous email, empty column, ISO date, entity demotion
- All 151 existing column.rs tests pass unchanged (backward compatibility)
- Full suite: 252 pass (finetype-model), all workspace tests pass
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
