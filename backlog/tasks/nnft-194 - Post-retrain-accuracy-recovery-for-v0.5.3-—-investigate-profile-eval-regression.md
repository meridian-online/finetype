---
id: NNFT-194
title: >-
  Post-retrain accuracy recovery for v0.5.3 — investigate profile eval
  regression
status: To Do
assignee: []
created_date: '2026-03-03 22:37'
labels:
  - accuracy
  - post-release
  - v0.5.3
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Investigate and resolve profile eval regression from v0.5.2 retrain.

**Regression summary:**
- v0.5.1 baseline: 117/119 (98.3% label), 119/119 (100% domain)
- v0.5.2 (after char-cnn-v10 retrain): 110/116 (94.8% label), 110/116 (94.8% domain)
- 3 columns removed from eval (unknown reason); 6 misclassifications

**Misclassifications (new in v0.5.2):**
1. utc_offset → excel_format (new)
2. ean → credit_card_number (new)
3. multilingual.name → region (new)
4. countries.sub-region → full_name (new)
5. countries.name → full_name (pre-existing, regression)
6. world_cities.name → full_name (new)

**Root cause:** CharCNN v10 retrain with 163-type taxonomy produced boundary shifts in decision space, not logic changes.

**Investigation approach:**
1. Compare CharCNN v9 vs v10 predictions on regression dataset (6 misclassifications)
2. Check if v9 predictions were correct and v10 regressed, or both wrong
3. Examine vote distributions for these columns — did masking/ranking change?
4. Determine if retrain is recoverable (model architecture) or requires taxonomy/pipeline adjustment
5. Consider per-type confidence thresholds or post-hoc rules to correct known misclassifications

**Non-blocking:** Profile eval at 94.8% is acceptable for release; regression documented in CHANGELOG. This task is follow-up for v0.5.3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Root cause analysis: v9 vs v10 predictions compared on regression dataset
- [ ] #2 Vote distribution analysis: confirm masking/ranking changes or other pipeline effects
- [ ] #3 Mitigation strategy identified: retrain, threshold tuning, or post-hoc rules
- [ ] #4 Plan documented for implementation in v0.5.3
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
