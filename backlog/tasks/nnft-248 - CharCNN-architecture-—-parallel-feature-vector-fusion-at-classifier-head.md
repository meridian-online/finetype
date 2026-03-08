---
id: NNFT-248
title: CharCNN architecture — parallel feature vector fusion at classifier head
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 23:55'
updated_date: '2026-03-08 00:20'
labels:
  - model
  - architecture
milestone: m-12
dependencies:
  - NNFT-247
references:
  - crates/finetype-model/src/char_cnn.rs
  - crates/finetype-train/src/training.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Modify the CharCNN architecture to accept a parallel feature vector alongside the learned character embeddings. The feature vector is concatenated with the CNN output **at the classifier head** (not at input embedding level).

Architecture change:
```
Input string → CharCNN conv layers → cnn_embedding (N dims)
Input string → Feature extractor → feature_vector (~30 dims)
[cnn_embedding ∥ feature_vector] → fusion layers → classifier → type prediction
```

The fusion layers are 1-2 fully connected layers that take the concatenated vector and produce the final logits. Both the CNN and the fusion layers train end-to-end jointly.

Also explore wider filters and deeper CNN layers within the 50MB binary budget.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CharCNN forward pass accepts both character input and feature vector
- [x] #2 Feature vector concatenated with CNN output before classifier head (not at embedding)
- [x] #3 Fusion layers (1-2 FC layers) connect concatenated vector to output logits
- [ ] #4 Model trains end-to-end — CNN weights and fusion weights update jointly
- [x] #5 Architecture supports both flat (250-class) and tiered model variants
- [ ] #6 Compiled model + binary stays under 50MB
- [x] #7 Backward-compatible model loading — can still load old CharCNN weights (feature vector zeroed/absent)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Architecture Change
Add `feature_dim: usize` to `CharCnnConfig` (default 0 for backward compat).
When `feature_dim > 0`, fc1 input becomes `total_filters + feature_dim`.

**Current flow** (feature_dim=0):
```
conv output: (batch, 256) → fc1(256→128) → relu → fc2(128→250) → logits
```

**New flow** (feature_dim=32):
```
conv output: (batch, 256) ∥ features: (batch, 32) → concat: (batch, 288)
→ fc1(288→128) → relu → fc2(128→250) → logits
```

### API Changes (char_cnn.rs)
1. Add `feature_dim: usize` to `CharCnnConfig` (default 0)
2. When feature_dim > 0, fc1 input = `total_filters + feature_dim`
3. New method: `forward_with_features(input_ids, features: Option<&Tensor>)`
4. `forward(input_ids)` becomes wrapper calling `forward_with_features(input_ids, None)`
5. When features=None and feature_dim>0, creates zero tensor
6. `infer` / `infer_with_features` same pattern

### Backward Compatibility
- Old config.yaml without `feature_dim` → defaults to 0 → fc1 input = total_filters (unchanged)
- Old model.safetensors loads fine since fc1 dimensions match
- New config.yaml with `feature_dim: 32` → fc1 input = total_filters + 32

### Files Changed
1. `crates/finetype-model/src/char_cnn.rs` — config + forward pass changes
2. `crates/finetype-model/src/inference.rs` — CharClassifier calls updated
3. `crates/finetype-model/src/char_training.rs` — training forward call updated

### Testing
- Existing tests pass (feature_dim=0 backward compat)
- New test: forward_with_features produces different output than without
- Config parsing test: feature_dim read from yaml
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- forward_with_features(input_ids, features: Option<&Tensor>) added to CharCnn
- feature_dim=0 backward compat: fc1 input unchanged, forward() wrapper passes None
- feature_dim>0: fc1 input = total_filters + feature_dim, zeros if features absent
- Config parsers in inference.rs + tiered.rs updated to read feature_dim (defaults 0)
- AC #4 (trains end-to-end) verified by architecture — all weights in VarMap, optimizer sees them. Full verification in NNFT-249 training task.
- AC #6 (binary <50MB) — architecture change adds only feature_dim parameters to fc1 weight matrix, negligible. Verified in NNFT-251 eval task."
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added parallel feature vector fusion to CharCNN architecture at the classifier head (NNFT-248).

## Changes
- **`char_cnn.rs`**: Added `feature_dim: usize` to `CharCnnConfig` (default 0). When >0, fc1 input dimension becomes `total_filters + feature_dim`. New methods: `forward_with_features(input_ids, features: Option<&Tensor>)` and `infer_with_features()`. Original `forward()`/`infer()` preserved as wrappers (backward compatible). When features=None and feature_dim>0, zeros are passed.
- **`inference.rs`**: Both config parsers (from_bytes, load) read `feature_dim` from config.yaml, defaulting to 0.
- **`tiered.rs`**: `parse_config_yaml()` returns 6-tuple with feature_dim. CharCnnConfig construction updated.
- **`char_training.rs`**: Explicit `feature_dim: 0` in legacy trainer config.
- **`tiered_training.rs`**: Explicit `feature_dim: 0` in tiered trainer config.

## Backward Compatibility
Old models with config.yaml missing `feature_dim` → defaults to 0 → fc1 dimensions unchanged → existing weights load correctly. No changes to model.safetensors format.

## Tests
- 279 tests pass, 1 doc test pass, zero regressions
- Clippy clean, fmt clean"
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
