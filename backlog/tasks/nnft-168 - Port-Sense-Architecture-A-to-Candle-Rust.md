---
id: NNFT-168
title: Port Sense Architecture A to Candle/Rust
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-28 23:06'
updated_date: '2026-03-01 00:00'
labels:
  - sense-sharpen
  - feature
dependencies:
  - NNFT-165
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement SenseClassifier in a new sense.rs module. Ports PyTorch SenseModelA (cross-attention over Model2Vec) to Candle. Loads safetensors weights matching the Phase 1 spike model. Includes BroadCategory/EntitySubtype enums and SenseResult struct.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 New sense.rs module with SenseClassifier struct
- [x] #2 from_bytes() loads safetensors with correct weight mapping (QKV split from in_proj)
- [x] #3 classify() produces correct BroadCategory and EntitySubtype
- [x] #4 Forward pass matches PyTorch output within 1e-4 tolerance on test inputs
- [x] #5 BroadCategory and EntitySubtype enums with Display/From impls
- [x] #6 Unit tests with synthetic tensors
- [x] #7 Integration test loads real spike model artifacts and classifies sample columns
- [x] #8 Exported from lib.rs
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Create sense.rs with BroadCategory, EntitySubtype enums and SenseResult struct
2. Implement SenseClassifier struct with cross-attention forward pass:
   - from_bytes(): load safetensors, split in_proj_weight into Q/K/V [128,128] each
   - classify(resources, header, values): encode via Model2VecResources, run forward pass
   - Forward: header_proj → cross_attention(4 heads) → layer_norm → cat(attn,mean,std) → MLP heads
3. Unit tests with synthetic 4-dim tensors
4. Integration test with real spike model artifacts
5. Export from lib.rs
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Ported PyTorch SenseModelA (Architecture A, cross-attention over Model2Vec) to Candle/Rust as SenseClassifier in sense.rs.

Changes:
- New sense.rs module (~580 lines) with BroadCategory (6 classes), EntitySubtype (4 classes) enums, SenseResult struct, and SenseClassifier
- from_bytes() loads safetensors with correct QKV split from PyTorch nn.MultiheadAttention in_proj_weight [3D,D] → Q/K/V [D,D] each
- classify() encodes header (unnormalised) and values via Model2VecResources, runs 4-head cross-attention → LayerNorm → feature concat → dual MLP heads
- Multi-head attention implements scaled dot-product with key padding mask for variable-length columns
- layer_norm() and masked_mean_std() helper methods
- Header encoded with encode_batch (unnormalised) not encode_one (L2-normalised), matching Python training pipeline
- Exported BroadCategory, EntitySubtype, SenseClassifier, SenseResult from lib.rs

Tests:
- 6 unit tests for enum Display/FromStr/from_index
- 2 unit tests for layer_norm and masked_mean_std with synthetic tensors
- 1 integration test loads real spike model artifacts and verifies forward pass on 6 column types
- Numerical equivalence verified against PyTorch: date=format(93.1%), email=entity(98.4%), name=entity(100%) — all match Python within <0.1%
- Full test suite: 237 tests pass

Key discovery: encode_one() L2-normalises (designed for semantic hint cosine similarity), but Sense was trained on unnormalised Model2Vec embeddings. Fixed by using encode_batch for header encoding.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
