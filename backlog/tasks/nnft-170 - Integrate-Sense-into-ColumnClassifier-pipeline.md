---
id: NNFT-170
title: Integrate Sense into ColumnClassifier pipeline
status: To Do
assignee: []
created_date: '2026-02-28 23:06'
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
- [ ] #1 ColumnClassifier gains sense, model2vec, label_map optional fields
- [ ] #2 classify_sense_sharpen() implements the full Sense→Sharpen pipeline
- [ ] #3 Masked vote aggregation with fallback to unmasked when all votes filtered
- [ ] #4 Entity demotion via Sense replaces EntityClassifier Rule 18
- [ ] #5 Header hints bypassed when Sense is active
- [ ] #6 When sense is None, classify_column and classify_column_with_header unchanged
- [ ] #7 All existing column.rs tests pass unchanged (fallback path)
- [ ] #8 New tests for Sense pipeline path
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
