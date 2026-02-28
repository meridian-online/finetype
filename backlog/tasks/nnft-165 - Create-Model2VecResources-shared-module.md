---
id: NNFT-165
title: Create Model2VecResources shared module
status: To Do
assignee: []
created_date: '2026-02-28 23:06'
labels:
  - sense-sharpen
  - refactor
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extract a shared Model2VecResources struct that owns the tokenizer and embedding matrix, loaded once and borrowed by SemanticHintClassifier, EntityClassifier, and SenseClassifier. Pure refactor with zero behavioural change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 New module model2vec_shared.rs with Model2VecResources struct
- [ ] #2 from_bytes() and load() constructors
- [ ] #3 encode_one() for single string → normalised embedding
- [ ] #4 encode_batch() for multiple strings → [N, D] matrix
- [ ] #5 Exported from lib.rs
- [ ] #6 Unit tests for encoding correctness
- [ ] #7 cargo test passes
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
