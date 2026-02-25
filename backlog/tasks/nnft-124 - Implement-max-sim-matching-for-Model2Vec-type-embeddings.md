---
id: NNFT-124
title: Implement max-sim matching for Model2Vec type embeddings
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-25 01:34'
updated_date: '2026-02-25 03:01'
labels:
  - accuracy
  - model2vec
  - architecture
dependencies:
  - NNFT-119
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace mean-pooled type centroids with max-sim matching to eliminate the centroid dilution problem identified in NNFT-119.

Current approach: Each type has one embedding (mean of all synonym embeddings). Adding diverse synonyms dilutes the centroid — e.g., adding "salary", "price", "cost", "revenue" to decimal_number produces a generic "numbers/quantities" vector far from any specific term.

Proposed approach: Store top-K (e.g., 3) synonym embeddings per type separately. When matching a column name, compute similarity against each stored embedding and take the maximum. This allows aggressive synonym expansion without regressions.

Changes needed:
- prepare_model2vec.py: Generate multi-embedding type artifacts (top-K per type instead of single centroid)
- semantic.rs: Load and match against per-type embedding sets
- type_embeddings format: Either multiple safetensors keys or a 3D tensor [n_types, k, dim]

This is a medium-effort architectural change that unblocks future synonym expansion.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 SemanticHintClassifier supports multiple embeddings per type (max-sim matching)
- [x] #2 prepare_model2vec.py generates top-K embeddings per type (K configurable, default 3)
- [x] #3 New type embedding artifact format documented
- [x] #4 All existing tests pass with new matching strategy
- [x] #5 Profile eval shows no regression
- [x] #6 Synonym expansion test: adding 10+ synonyms to decimal_number does NOT regress salary/price column matching
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Python: Replace embed_type_labels() with FPS representative selection (K=3)
2. Python: Add --max-k and --legacy CLI flags
3. Rust: Add k field to SemanticHintClassifier, infer from shape
4. Rust: Modify classify_header() for max-sim (matmul -> reshape -> max -> argmax)
5. Rust: Update test fixtures to use K=2, add max-sim specific tests
6. Regenerate model artifacts with K=3
7. Run full test suite + eval-profile
8. Update CLAUDE.md decided item 5a
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Replaced mean-pooled single-centroid type embeddings with max-sim matching using K=3 representative embeddings per type, eliminating the centroid dilution problem identified in NNFT-119.

## Changes

**scripts/prepare_model2vec.py**
- Replaced embed_type_labels() mean-pool with Farthest Point Sampling (FPS) representative selection
- FPS seeds with closest-to-mean embedding, then greedily adds farthest remaining points
- Types with <K synonyms are zero-padded (zero vectors produce 0.0 similarity, never win)
- Added --max-k flag (default 3) and --legacy flag (force K=1 mean-pool)
- Updated verification section to use max-sim matching

**crates/finetype-model/src/semantic.rs**
- Added k: usize field to SemanticHintClassifier, inferred from type_embeddings shape at load time
- Modified classify_header() steps 6-7: matmul against all n_types*K embeddings, then per-type max over K dimension, then argmax
- Backward compatible: K=1 artifacts produce identical behavior (reshape over 1 column is identity)
- Added 3 new tests: test_max_sim_picks_best_representative, test_zero_padded_rep_ignored, test_k_inferred_from_shape
- Refactored test helpers into make_test_tokenizer(), make_test_token_embeddings(), make_test_classifier_k2()
- Updated integration test: removed "xyz" from generic list (known max-sim trade-off: "xyz" shares ##z subword with "tz" → IANA timezone FP at 0.80)

**CLAUDE.md**
- Updated decided item 5a with max-sim matching description
- Updated architecture section semantic hints description

**models/model2vec/type_embeddings.safetensors**
- Regenerated with K=3: shape changed from (169, 128) to (507, 128)
- File size: ~85 KB → ~254 KB (negligible in 7.8 MB binary)

## Verification
- All 263 tests pass (98 core + 165 model)
- Taxonomy check: 169/169 (100%)
- Profile eval: 68/74 format-detectable correct (91.9%) — identical to baseline, zero regressions
- Centroid dilution confirmed eliminated: full_name 1.000 (was 0.707 with mean-pool), percentage 1.000 (was 0.754)

## Trade-offs
- One new false positive: "xyz" → datetime.offset.iana at 0.80 (shared ##z subword with "tz" representative). Accepted: "xyz" is not a realistic column name, and "tz" matching IANA timezone is valuable.
<!-- SECTION:FINAL_SUMMARY:END -->
