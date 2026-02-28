---
id: NNFT-166
title: Refactor SemanticHintClassifier to use shared Model2Vec
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-28 23:06'
updated_date: '2026-02-28 23:44'
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
- [x] #1 from_shared() constructor accepts Model2VecResources ref
- [x] #2 Internally clones tokenizer and embeddings (O(1) Arc-based)
- [x] #3 Existing load() and from_bytes() still work
- [x] #4 All semantic.rs tests pass unchanged
- [x] #5 Integration test confirms shared resources match standalone loading
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add from_shared() constructor to SemanticHintClassifier
   - Takes &Model2VecResources + type_emb_bytes + label_bytes
   - Clones tokenizer and embeddings from shared resources (O(1) Arc-based)
   - Builds type_embeddings and label_index as before
2. Add integration test: load via from_shared() and verify identical behaviour to load()
3. Keep existing load() and from_bytes() unchanged
4. cargo test — all existing semantic.rs tests pass
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added `from_shared()` constructor to `SemanticHintClassifier` that accepts `&Model2VecResources` instead of loading its own tokenizer/embeddings.

Changes:
- New `from_shared(resources, type_emb_bytes, label_bytes)` constructor clones tokenizer and embeddings from shared resources (O(1) Arc-based Tensor clone)
- Extracted `load_type_embeddings()` helper to eliminate duplication between `from_bytes()` and `from_shared()`
- `from_bytes()` now delegates to `Model2VecResources::from_bytes()` + `from_shared()` internally
- Existing `load()` and `from_bytes()` API unchanged — full backward compatibility

Tests:
- All 12 existing semantic.rs tests pass unchanged
- New integration test `test_from_shared_matches_load` verifies identical labels and similarities for 6 headers
- Full suite: 227 tests pass
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
