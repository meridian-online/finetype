# Implementation Progress

**Spec:** specs/2026-04-11-candle-autoresearch-port/spec.yaml
**Started:** 2026-04-11

## Hard Constraints
- [x] Production-scale config dimensions — do not change hidden sizes
- [x] Two primary files: finetype-train + finetype-model multi_branch.rs
- [x] Backward compatible via #[serde(default)]
- [x] Header branch retained
- [ ] Mac Metal training
- [ ] Profile eval >=160/190 to publish
- [ ] No regression in actionability eval
- [x] Training config: GELU, use_layer_norm=true, weight_decay=0.01, lr=0.001

## Acceptance Criteria
- [x] ac-01: Activation enum in both crates with serde default ReLU — Activation enum added to both crates
- [x] ac-02: use_layer_norm field in both crates with serde default false — field added with #[serde(default)]
- [x] ac-03: Configurable activation in training forward pass — BranchWeights.activate() + forward_trunk closure
- [x] ac-04: Optional input LayerNorm on all branches (training) — build_trunk uses new_with_input_norm when use_layer_norm=true
- [x] ac-05: Conditional merge LayerNorm/BatchNorm (training) — MergeNorm enum, LayerNorm when use_layer_norm=true
- [x] ac-06: Configurable activation in inference forward pass — BranchWeights.activate() + forward_trunk closure
- [x] ac-07: Optional input LayerNorm in inference BranchWeights — new_with_input_norm for all branches when use_layer_norm=true
- [x] ac-08: Conditional merge LayerNorm/BatchNorm (inference) — MergeNorm enum, LayerNorm when use_layer_norm=true
- [x] ac-09: Training config defaults (weight_decay=0.01, lr=0.001) — updated in MultiBranchTrainConfig::default()
- [x] ac-10: Tests for both old-style and new-style configs — 5 new tests added, 22 training + 5 inference all pass
- [ ] ac-11: Train new model on Mac with Metal
- [ ] ac-12: Profile eval >=160/190
- [ ] ac-13: No actionability regression
- [ ] ac-14: Publish to HuggingFace

## Test Results

### finetype-train (22 tests, all pass)
- test_forward_pass_shape (ReLU, existing)
- test_gradient_flow (ReLU, existing)
- test_forward_pass_shape_gelu_layer_norm (NEW)
- test_gradient_flow_gelu_layer_norm (NEW)
- test_gelu_vs_relu_outputs_differ (NEW)
- test_config_serialization (updated)
- test_config_serialization_gelu_layer_norm (NEW)
- test_config_backward_compat_deserializes_without_new_fields (NEW)
- ... plus 14 existing tests unchanged

### finetype-model (5 tests, all pass)
- test_config_deserialization (updated: verifies activation=ReLU, use_layer_norm=false defaults)
- test_config_deserialization_gelu_layer_norm (NEW)
- test_config_deserialization_with_header (existing)
- test_config_deserialization_hierarchical (existing)
- test_is_multi_branch_dir_missing_files (existing)
