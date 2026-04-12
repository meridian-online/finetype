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
- [ ] ac-12: Profile eval >=160/190 — GELU+LN does not meet threshold. Best: v10 188/227 (82.8%) vs baseline 193/227 (85.0%). Experiment closed.
- [x] ac-13: No actionability regression — v10: 97.0% vs baseline 96.9%. PASS.
- [ ] ac-14: Publish to HuggingFace — not applicable; GELU+LN not adopted

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

## v9 Results (GELU+LN, lr=0.0001, wd=0.0001) — HYPOTHESIS REJECTED

| Model | Label | Domain | Actionability | Val Acc | Best Epoch |
|-------|-------|--------|---------------|---------|------------|
| v4-sibling (baseline) | 179/214 (83.6%) | 197/214 (92.1%) | 96.9% | — | — |
| v6-gelu (v8, lr=1e-3) | 167/214 (78.0%) | 187/214 (87.4%) | 94.5% | 84.63% | 27 |
| v6-gelu-cons (v9, lr=1e-4) | 166/214 (77.6%) | 191/214 (89.3%) | 97.0% | 85.93% | 29 |

**Hypothesis rejected:** Conservative LR scored 166/214 — same regression as v8 (167/214).
LR is not the cause. The GELU+LN architecture itself regresses profile eval by ~13 labels.

Paradox: v9 has *better* val_accuracy (85.9% vs 84.6%) but *worse* profile eval.
This confirms val_accuracy and profile eval measure different things — profile eval includes
Sharpen post-processing, which the GELU+LN output distribution doesn't interact well with.

v8↔v9 diff: 40 both wrong, v9 fixed 7, v9 broke 8. Nearly identical failure modes.
Both share: decimal→dmy_short_dot, person→email_display, phone→ssn patterns.

### Root Cause Analysis — REVISED (v8/v9 comparison was invalid)

**Finding:** v8/v9 trained on `v7-blend-50-50.ftmb` which has ALL-ZERO header features
and empty sibling strings. The header branch learned nothing — effectively a 3-branch model.
v4-sibling trained on v3 data with real Model2Vec headers + frozen sibling-context enrichment
(true 4-branch model). The 8-label gap may be explained by missing header signal, not GELU+LN.

Evidence from v8 training log:
```
Record 0 [group 0, col_idx=0, header=""]: representation.text.entity_name
  siblings: ["", "", "", "", "", "", "", "", "", "", "", "", "", ""]
  header : dim=128, nonzero=0, range=[0.0000, 0.0000], mean=0.0000
```

**Architecture audit:** GELU+LN properly propagates activation to all 4 branches including
header. `FrozenSiblingContext` is architecture-agnostic. No code changes needed.

**v10 plan:** Retrain GELU+LN on fresh FTMB with real headers and sibling enrichment.
Script: `scripts/overnight_v10_gelu_headers.sh` — includes header feature validation
that hard-fails if Model2Vec isn't producing non-zero embeddings.

## v10 Results (GELU+LN, real headers, fair comparison) — CLOSED

First fair apples-to-apples comparison: both models trained on FTMB v3 with real
Model2Vec header features and frozen sibling-context enrichment during training.

| Model | Label | Domain | Actionability |
|-------|-------|--------|---------------|
| v4-sibling (ReLU+BN) | 193/227 (85.0%) | 206/227 (90.7%) | 96.9% |
| v10-gelu (GELU+LN) | 188/227 (82.8%) | 203/227 (89.4%) | 97.0% |
| **Delta** | **-5 (-2.2%)** | **-3 (-1.3%)** | **+0.1%** |

### Decomposing the gap across experiments

| Experiment | vs baseline | Header data | What it isolated |
|------------|-------------|-------------|------------------|
| v8 (lr=1e-3) | -8 labels | zeros | Architecture + LR + headers confounded |
| v9 (lr=1e-4) | -8 labels | zeros | Ruled out LR; architecture + headers confounded |
| v10 (lr=1e-4, real headers) | **-5 labels** | real | **Isolated architecture effect** |

3 of the original 8-label gap was the dead header branch. The remaining **5 labels
are a genuine GELU+LN regression** at this model scale and training data mix.

### Conclusion

GELU+LN does not improve profile eval accuracy over ReLU+BN for the multi-branch
architecture at production scale. The experiment is closed. v4-sibling remains the
default model. Future accuracy work should focus on training data quality and mix
(decision 0038: retraining > new rules).

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
