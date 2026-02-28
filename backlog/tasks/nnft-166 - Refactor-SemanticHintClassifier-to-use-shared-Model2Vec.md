---
id: NNFT-166
title: Refactor SemanticHintClassifier to use shared Model2Vec
status: To Do
assignee: []
created_date: '2026-02-28 23:06'
labels:
  - sense-sharpen
  - refactor
dependencies:
  - NNFT-165
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add from_shared() constructor to SemanticHintClassifier that accepts Model2VecResources instead of loading its own tokenizer/embeddings. Keep existing load() and from_bytes() methods for backward compatibility. Pure refactor — all existing tests must pass unchanged.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 from_shared() constructor accepts Model2VecResources ref
- [ ] #2 Internally clones tokenizer and embeddings (O(1) Arc-based)
- [ ] #3 Existing load() and from_bytes() still work
- [ ] #4 All semantic.rs tests pass unchanged
- [ ] #5 Integration test confirms shared resources match standalone loading
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
