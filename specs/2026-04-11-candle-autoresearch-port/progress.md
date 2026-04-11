# Implementation Progress

**Spec:** specs/2026-04-11-candle-autoresearch-port/spec.yaml
**Started:** 2026-04-11

## Hard Constraints
- [x] Production-scale config dimensions — do not change hidden sizes
- [x] Two primary files: finetype-train + finetype-model multi_branch.rs
- [x] Backward compatible via #[serde(default)]
- [x] Header branch retained
- [x] Mac Metal training — v8 completed 30 epochs in 1h08m on M1 Pro
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
- [x] ac-11: Train new model on Mac with Metal — v8 completed, 30 epochs, best val_accuracy 84.6% at epoch 27
- [ ] ac-12: Profile eval >=160/190 — v8 FAILED: 167/214 (78.0%) vs baseline 179/214 (83.6%), -12 regression
- [ ] ac-13: No actionability regression — v8: 94.5% vs baseline 96.9%, minor regression
- [ ] ac-14: Publish to HuggingFace — blocked by ac-12

## v8 Results (GELU+LN, lr=0.001, wd=0.01) — REGRESSION

| Model | Label | Domain | Actionability |
|-------|-------|--------|---------------|
| v4-sibling (baseline) | 179/214 (83.6%) | 197/214 (92.1%) | 96.9% |
| v6-gelu (v8) | 167/214 (78.0%) | 187/214 (87.4%) | 94.5% |
| **Delta** | **-12 (-5.6%)** | **-10 (-4.7%)** | **-2.4%** |

19 regressions, 7 fixes, 28 both wrong. Primary failure modes:
- decimal_number → dmy_short_dot/ip_v4/icd10 (7 regressions)
- person names → email_display (3 regressions)

**Exit condition triggered:** score < baseline → revert and investigate.

## v9 Experiment (in progress): Isolating Architecture vs Hyperparameters

**Hypothesis:** Regression caused by 10x LR (1e-4 → 1e-3), not GELU+LN architecture.
**Config:** Same GELU+LN architecture, but lr=0.0001, wd=0.0001 (original values).
**Script:** `scripts/overnight_v9_conservative.sh`

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
