---
id: NNFT-165
title: Create Model2VecResources shared module
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-28 23:06'
updated_date: '2026-02-28 23:39'
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
- [x] #1 New module model2vec_shared.rs with Model2VecResources struct
- [x] #2 from_bytes() and load() constructors
- [x] #3 encode_one() for single string → normalised embedding
- [x] #4 encode_batch() for multiple strings → [N, D] matrix
- [x] #5 Exported from lib.rs
- [x] #6 Unit tests for encoding correctness
- [x] #7 cargo test passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Create crates/finetype-model/src/model2vec_shared.rs with Model2VecResources struct
   - Fields: tokenizer (Tokenizer), embeddings (Tensor), device (Device)
   - load(model_dir) loads tokenizer.json + model.safetensors from disk
   - from_bytes(tokenizer_bytes, model_bytes) loads from in-memory slices
   - encode_one(text) → Option<Tensor> — tokenize, filter PAD, index_select, mean pool, L2 normalize (returns [D])
   - encode_batch(texts) → Tensor — batch encode returning unnormalised [N, D] (matching entity classifier needs)
   - embed_dim(), tokenizer(), embeddings() accessors
2. Add unit tests: encode_one correctness, encode_batch shape, empty input, PAD filtering, L2 normalisation
3. Register module in lib.rs, export Model2VecResources
4. cargo test to verify no regressions (319 existing tests + new tests pass)
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Created `Model2VecResources` shared module to own the tokenizer and embedding matrix loaded once, replacing the current pattern where `SemanticHintClassifier` owns these resources and `EntityClassifier` clones them.

Changes:
- New `crates/finetype-model/src/model2vec_shared.rs` (~160 lines implementation + ~160 lines tests)
- `Model2VecResources` struct with `load()` and `from_bytes()` constructors
- `encode_one(text)` → L2-normalised embedding [D] (for cosine similarity use cases)
- `encode_batch(texts)` → unnormalised [N, D] matrix (for entity/sense feature computation)
- `embed_dim()`, `tokenizer()`, `embeddings()`, `device()` accessors
- Registered in `lib.rs`, exported as `finetype_model::Model2VecResources`

This is purely additive — no existing code changed. Consumers (SemanticHintClassifier, EntityClassifier) will be refactored to use `Arc<Model2VecResources>` in NNFT-166 and NNFT-167.

Tests:
- 12 new unit tests covering: encode correctness, batch shapes, empty input, PAD filtering, L2 normalisation, UNK handling, encode_one vs encode_batch consistency
- Integration test loads real potion-base-4M artifacts from disk (128-dim)
- All 226 tests pass, taxonomy check passes
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
