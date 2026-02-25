---
id: NNFT-126
title: 4-level locale labels with model retraining
status: To Do
assignee: []
created_date: '2026-02-25 03:31'
labels:
  - accuracy
  - locale
  - model-training
dependencies:
  - NNFT-121
  - NNFT-118
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generate training data with locale suffix labels (e.g., geography.address.postal_code.EN_US) and retrain CharCNN on the expanded ~484-class label set.

This is the high-risk, high-reward phase of locale intelligence:
1. Implement generate_all_localized() in generator.rs for locale-suffixed training data
2. Retrain CharCNN on expanded label set
3. Update inference pipeline to collapse 4-level predictions to 3-level user labels with locale metadata
4. Evaluate accuracy — no regressions allowed on profile eval

Depends on locale validation patterns (NNFT-118, NNFT-121) being proven in production first. May also benefit from CLDR data foundation (NNFT-060).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 4-level training data generated via generate_all_localized()
- [ ] #2 Model retrained on locale-expanded labels with acceptable accuracy
- [ ] #3 Inference pipeline returns 3-level user label with locale as metadata field
- [ ] #4 No regression on profile eval
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
