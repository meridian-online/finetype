---
id: NNFT-167
title: Refactor EntityClassifier to use shared Model2Vec
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-28 23:06'
updated_date: '2026-02-28 23:47'
labels:
  - sense-sharpen
  - refactor
dependencies:
  - NNFT-165
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add from_shared() constructor to EntityClassifier that accepts Model2VecResources instead of taking owned tokenizer/embeddings. Keep existing from_bytes() with owned args for backward compatibility. Pure refactor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 from_shared() constructor accepts Model2VecResources ref
- [x] #2 Existing from_bytes() and load() still work
- [x] #3 All entity.rs tests pass unchanged
- [x] #4 Integration test confirms shared resources match standalone loading
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add from_shared() constructor to EntityClassifier
   - Takes &Model2VecResources + model_bytes + config_bytes
   - Clones tokenizer and embeddings from shared resources
2. Add integration test: from_shared() matches existing load() path
3. Keep existing from_bytes() and load() unchanged
4. cargo test — all entity.rs tests pass
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added `from_shared()` and `load_shared()` constructors to `EntityClassifier` that accept `&Model2VecResources` instead of owned tokenizer/embeddings clones.

Changes:
- New `from_shared(model_bytes, config_bytes, resources)` constructor delegates to existing `from_bytes()` with cloned tok/emb
- New `load_shared(model_dir, resources)` convenience constructor for disk-based loading
- Existing `load()` and `from_bytes()` API unchanged — full backward compatibility

Tests:
- All 5 existing entity.rs tests pass unchanged
- New integration test `test_from_shared_matches_load` verifies identical demotion decisions for person and org columns
- Full suite: 333 tests pass
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
