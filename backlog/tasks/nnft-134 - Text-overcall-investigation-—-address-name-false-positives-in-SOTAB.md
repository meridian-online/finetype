---
id: NNFT-134
title: Text overcall investigation — address/name false positives in SOTAB
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-25 09:50'
updated_date: '2026-02-25 18:01'
labels:
  - accuracy
  - discovery
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
619 SOTAB columns: generic text misclassified as full_address (341) or full_name (278). Book titles, summaries, event names being misclassified. Needs investigation: attractor addition? confidence tuning? model retraining? Root cause analysis to determine fix path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Sample SOTAB text overcall columns analysed — identify pattern of false positives
- [x] #2 Root cause determined: attractor gap vs confidence threshold vs model confusion
- [x] #3 Written finding with data: which fix path gives the most recovery
- [x] #4 Follow-up implementation task created if fix is viable
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Investigation Findings

### Scale of Problem
- 5,243 columns predicted as full_name (3,544) or full_address (1,699)
- Only 741 legitimate (14%) — 4,502 overcall (86%)
- 1,704 overcall columns in format-detectable tier (14.8% of total)
- This is THE single largest error source in SOTAB

### Root Cause: Model Training Data Bias
- CharCNN trained on person names → classifies entity names (songs, restaurants, books) as full_name
- CharCNN trained on addresses → classifies long text (descriptions, recipes, paragraphs) as full_address
- No clean confidence threshold — even at 0.9+ confidence, 71-72% of predictions are wrong
- Fundamental training data / taxonomy gap: no "entity name" or "free text" class

### Structural Analysis
- full_address: overcall median value length 53.5 vs correct 23.0 — strong separation
- full_name: overcall median 20.5 vs correct 14.0 — weak separation (overlap at p25)
- Length-based demotion viable for full_address, not for full_name

### Fix Path Assessment
1. full_address length rule (median > 80): 524 true demotions, 3 false. +500 columns recovered. Safe.
2. full_name: no surgical rule available. Needs model retraining (NNFT-126).
3. Domain impact of demotion to representation: +861 domain gains, -176 domain losses, net +685.

Decision: full_address length rule at threshold 100 (0% false demotion). Skip to NNFT-126 retraining after.

## Implementation Result

Implemented full_address text length demotion rule (Rule 16):
- Threshold: median value length > 100 chars → demote to representation.text.sentence
- 441 columns demoted in SOTAB eval
- Domain accuracy: 62.6% → 64.4% (+1.8pp)
- Label accuracy: 42.5% unchanged (demoted type is not the exact expected label)
- Profile eval: 70/74 unchanged
- 178 model tests pass (4 new), taxonomy check clean

Follow-up: full_name overcall (3,086 columns) requires model retraining — no surgical rule available. This feeds directly into NNFT-126 (4-level locale labels with retraining). Updated NNFT-126 description to include entity name disambiguation as a retraining goal.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Investigated full_name/full_address text overcall — the single largest error source in SOTAB (5,243 columns, 86% false positive rate, 1,704 wrong in format-detectable tier).

## Root Cause
CharCNN training data bias: entity names (songs, restaurants, products) classified as person names, long text (descriptions, recipes) classified as addresses. No confidence threshold can fix this — 71% wrong even at 0.9+ confidence.

## Fix Implemented
Added Rule 16 (text_length_demotion_full_address) in column.rs: when full_address prediction has median value length >100 chars, demote to representation.text.sentence.
- 441 SOTAB columns corrected
- SOTAB domain accuracy: 62.6% → 64.4% (+1.8pp)
- Profile eval: 70/74 unchanged
- 4 new tests, 178 total pass

## Finding: full_name Overcall Needs Retraining
3,086 overcall columns have no surgical rule fix (length overlap too large). The model needs entity name classes and/or a free-text type. This feeds directly into NNFT-126 model retraining.

## Cumulative SOTAB Progress (v0.3.0 → now)
- Label: 30.5% → 42.5% (+12.0pp)
- Domain: 54.8% → 64.4% (+9.6pp)
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
