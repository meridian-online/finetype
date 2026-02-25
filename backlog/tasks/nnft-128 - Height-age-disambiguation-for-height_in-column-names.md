---
id: NNFT-128
title: Height/age disambiguation for height_in column names
status: To Do
assignee: []
created_date: '2026-02-25 03:38'
labels:
  - accuracy
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
medical_records.height_in is predicted as age at 0.967 confidence. The height header hint exists but does not match height_in (the _in suffix for inches). Model2Vec semantic matching should handle this — verify height_in similarity score. If below threshold, add height_in as a synonym in prepare_model2vec.py header hints.

File: crates/finetype-model/src/column.rs or scripts/prepare_model2vec.py
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 height_in correctly predicted as height (not age)
- [ ] #2 No regression on other height/age predictions
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
