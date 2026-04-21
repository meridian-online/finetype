# Spec Review

**Date:** 2026-04-11
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-11-candle-autoresearch-port/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] Inference crate not in scope — trained model will not load
**Category:** missing-requirement
**Description:** The spec's constraint says "Single primary file: crates/finetype-train/src/multi_branch.rs" and the deliverables list only that file. However, the inference crate (`crates/finetype-model/src/multi_branch.rs`) has its own independent `MultiBranchConfig`, `BranchWeights`, and `MultiBranchClassifier` structs with hardcoded ReLU activations, no `input_norm`/LayerNorm support on branches, and a hardcoded BatchNorm merge layer. A model trained with GELU+LayerNorm will produce incorrect results at inference time because the inference crate will apply ReLU and BatchNorm to weights that were trained with GELU and LayerNorm.
**Evidence:** `crates/finetype-model/src/multi_branch.rs` lines 99-104 show `BranchWeights::forward()` hardcodes `.relu()`. Lines 527-537 show the merge layer hardcodes BatchNorm with the unsqueeze/squeeze workaround. There is no `activation` or `use_layer_norm` field in the inference crate's `MultiBranchConfig` (lines 46-65). The inference crate's `BranchWeights` struct (line 87-90) has no `input_norm` field at all, and the `new()` constructor does not attempt to load LayerNorm weights.
**Recommendation:** Add deliverables and ACs for `crates/finetype-model/src/multi_branch.rs`. The inference-side `MultiBranchConfig` needs the same `activation` and `use_layer_norm` fields with serde defaults. `BranchWeights` needs optional `input_norm` support. The merge layer needs conditional LayerNorm/BatchNorm. Without this, profile eval (ac-09) will silently produce wrong results — the model will "load" but misclassify because the activation functions differ from training.

### [HIGH] Spec claims "5 improvements" but only specifies 3
**Category:** missing-requirement
**Description:** The spec goal and interview both reference "5 empirically validated architecture improvements" from the autoresearch. The ACs cover: (1) GELU activation, (2) LayerNorm replacing BatchNorm, (3) weight_decay increase 1e-4 → 0.01. The autoresearch `train.py` also uses: (4) learning rate 0.001 vs Candle default 1e-4 (10x increase), (5) cosine schedule with linear warmup (Candle has cosine but no warmup). Additionally, (6) `input_norm=True` on ALL branches (char, embed, stats) — whereas current Candle only has input LayerNorm on the header branch. AC-04 covers adding input_norm to char/embed/stats, so that's 4 improvements. LR and warmup are missing entirely.
**Evidence:** `research/train.py` line 30: `LEARNING_RATE = 0.001`. Candle default at `multi_branch.rs` line 1240: `lr: 1e-4`. `research/train.py` lines 117-128: warmup implementation. Candle `CosineScheduler` (line 1775) has no warmup. The interview's "Open Questions" section says "None — scope is tight" which suggests these omissions are unintentional rather than deliberate exclusions.
**Recommendation:** Either (a) enumerate all 5 improvements explicitly and add ACs for LR and warmup changes, or (b) reduce the claimed count to match what's actually specified. The LR change is likely impactful — a 10x difference in learning rate is not a minor detail.

### [HIGH] No AC for training config as a whole — only weight_decay default
**Category:** test-gap
**Description:** AC-06 changes the `weight_decay` default, but ac-08 says "Train new model on Mac with Metal using GELU+LayerNorm+weight_decay=0.01 config" without specifying how the training config is assembled. The training config (MultiBranchTrainConfig) is separate from the model config (MultiBranchConfig). There's no AC ensuring the right combination of model config + training config is used together. The actual training invocation (CLI flags? config file? hardcoded?) is unspecified.
**Evidence:** `MultiBranchTrainConfig` (line 1215) and `MultiBranchConfig` (line 47) are separate structs. AC-08 verification is just "Training completes without error. Model artifact produced" — it does not verify the correct architecture settings were applied.
**Recommendation:** AC-08 should specify the exact config: model config with `activation=GELU, use_layer_norm=true`, and training config with `weight_decay=0.01, lr=?`. The verification should confirm the saved `config.json` in the model artifact contains the expected values.

### [MEDIUM] Profile eval may pass with wrong inference path
**Category:** failure-mode
**Description:** AC-09 verification is "make eval-report shows label accuracy >= 160/190." The profile eval script (`eval/profile_eval.sh`) uses the CLI, which uses the inference crate's `MultiBranchClassifier`. If the inference crate is not updated (see critical finding above), the eval could still run — it would just use ReLU+BatchNorm on GELU+LayerNorm-trained weights, producing degraded but potentially non-crashing results. The score might land at, say, 140/190 and correctly fail the gate, but the failure message would be misleading (looks like a model quality issue, not an architecture mismatch bug).
**Evidence:** `crates/finetype-model/src/multi_branch.rs` line 157: `MultiBranchClassifier::load()` loads config.json but ignores unknown fields (serde default behavior). If `activation` and `use_layer_norm` are in config.json but the inference struct doesn't have those fields, serde silently drops them — no error, just wrong behavior.
**Recommendation:** Add a verification step that explicitly checks the inference forward pass matches the training forward pass. For example: a unit test that creates a model in the training crate, saves it, loads it in the inference crate, runs the same input through both, and asserts matching output.

### [MEDIUM] Backward compatibility test insufficient
**Category:** test-gap
**Description:** AC-07 verification is "cargo test -p finetype-train succeeds." The existing tests (lines 2039-2232) only test the default config (ReLU, no input_norm on non-header branches, BatchNorm merge). There is no existing test that creates a model with the new GELU+LayerNorm config and verifies it works. The AC says "both old-style and new-style configs" but the verification method doesn't ensure new tests are actually added.
**Evidence:** Existing tests at lines 2043-2070 (`test_forward_pass_shape`) and 2072-2153 (`test_gradient_flow`) use `MultiBranchConfig::default()` which produces the old-style config. If someone adds the new fields with defaults that happen to produce the same behavior as old code, these tests pass without exercising the new code paths at all.
**Recommendation:** AC-07 should require at least one test that explicitly sets `activation=GELU` and `use_layer_norm=true` and verifies forward pass shape, gradient flow, and output differs from the ReLU path.

### [MEDIUM] Model name "sherlock-v6-gelu" vs actual model naming
**Category:** assumption
**Description:** The deliverable names the model directory `models/sherlock-v6-gelu/`. CLAUDE.md references `sherlock-v4-sibling` as the current default and describes the default model as "sherlock-v4-sibling (4-branch: char+embed+stats+header)." The interview does not discuss model naming or versioning. The "v6" jump from "v4" is unexplained (where is v5?).
**Evidence:** Spec deliverable line 74: `models/sherlock-v6-gelu/`. CLAUDE.md: "sherlock-v4-sibling". Constraint line 7: "sherlock-v5-scaled" referenced for config dimensions, suggesting v5 exists or existed. No version lineage documented.
**Recommendation:** Minor, but clarify the model version lineage to avoid confusion. If v5-scaled is the production config and v6 is this experiment, say so explicitly.

### [MEDIUM] Exit condition gap: what if 155 < score < 160?
**Category:** gap-analysis
**Description:** Exit conditions say: (1) publish if >= 160/190, or (2) revert if < 155/190 (regression). There is no exit condition for the range 155-159. If the new model scores 158/190, it's better than baseline (155) but below publish threshold (160). The spec gives no guidance on what to do.
**Evidence:** Spec lines 88-89: exit conditions. Profile eval constraint line 12: ">= 160/190 label accuracy to publish." Interview answer Q5: ">= 160/190. Must beat current 155/190 by a meaningful margin."
**Recommendation:** Add an explicit exit condition: "If 155 <= score < 160: keep model artifact locally but do not publish to HuggingFace. Document as incremental improvement for next iteration."

### [LOW] Constraint "do not change hidden sizes" may conflict with autoresearch findings
**Category:** constraint-conflict
**Description:** Constraint 1 says "Production-scale config dimensions (sherlock-v5-scaled: [450,450]/[300,300]/[192,96]/[750,750]) — do not change hidden sizes." The current `MultiBranchConfig::default()` in the code uses `[300,300]/[200,200]/[128,64]/[500,500]` — which does NOT match the "production-scale" dimensions in the constraint. This means either the default needs changing (which is itself a config change the spec doesn't account for) or the constraint is describing a different config that's used at training time but not the code default.
**Evidence:** Spec constraint line 7. Code `MultiBranchConfig::default()` at lines 79-95. The numbers don't match: constraint says `[450,450]` for char but default is `[300,300]`.
**Recommendation:** Clarify: is the training config assembled from CLI flags or a config file that overrides the default? If so, the spec should include the full training config (or path to it) as a deliverable or AC to prevent ambiguity.

### [LOW] No rollback plan for HuggingFace publication
**Category:** gap-analysis
**Description:** AC-11 publishes the model to HuggingFace. The DuckDB extension downloads from there at runtime. If the published model has a subtle bug discovered post-publication (e.g., passes eval but fails on real-world data), there's no documented rollback procedure. Can the old model be restored? Is there versioning on HuggingFace?
**Evidence:** Spec deliverable line 77: "HuggingFace: meridian-online/finetype-model". No mention of versioning, tagging, or rollback.
**Recommendation:** Note the rollback plan: "HuggingFace model is published under a versioned tag. DuckDB extension pins to a specific version. Previous model remains available under old tag."

---

## Honest Assessment

This spec is well-scoped for what it explicitly covers — the training-side architecture changes are clear, backward compatibility via serde defaults is a sound approach, and the evaluation gates are concrete. However, the plan has a critical blind spot: the inference crate (`finetype-model/src/multi_branch.rs`) is a completely separate copy of the model architecture that also needs GELU, LayerNorm, and input normalization support, and it is entirely absent from the spec's scope, deliverables, and acceptance criteria. A model trained with this spec would load without errors in the inference path but silently apply wrong activation functions and normalization, producing degraded accuracy. This would likely be caught by the profile eval gate (ac-09), but the root cause would be confusing and time-consuming to diagnose. The second significant risk is the "5 improvements" claim not matching the 3-4 actually specified — the 10x learning rate difference between the autoresearch winner and the Candle default is likely material. I recommend addressing the inference crate gap and the improvement enumeration before starting implementation.
