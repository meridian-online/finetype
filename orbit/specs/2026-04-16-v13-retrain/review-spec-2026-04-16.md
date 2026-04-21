# Spec Review: v13 Retrain

**Reviewer:** Fresh context (no prior involvement)
**Date:** 2026-04-16
**Spec:** `orbit/specs/2026-04-16-v13-retrain/spec.yaml`
**Interview:** `orbit/specs/2026-04-16-v13-retrain/interview.md`
**Input:** `orbit/specs/2026-04-16-v12-data-quality-audit/retrain-brief.md`

**Verdict: REQUEST_CHANGES**

The spec is well-structured and the retrain brief provides strong evidence for each priority tier. However, there are several gaps — most critically around the n_classes change (239 to 240), backwards compatibility of the EpochMetrics struct, and missing specification of how distilled filtering integrates with the existing Python pipeline. None are blockers on their own, but collectively they create enough ambiguity to warrant changes before implementation.

---

## 1. Assumption Audit

### A1: n_classes stays at 239
**Where:** ac-06 says "merged dim changes from 1006 to 1070." ac-01 adds state_code as a new type.
**Problem:** The current taxonomy has 239 types (verified in `label_category_map.rs` test `test_total_is_239`). Adding `geography.location.state_code` makes it 240. The spec never mentions n_classes changing from 239 to 240. The merged_dim calculation of 1070 appears to assume valid_dim stays at 239 (450+300+96+96+128 = 1070), but if valid_dim becomes 240, merged_dim becomes 1071. More importantly:
- `n_classes` in config.json must change from 239 to 240
- `valid_dim` must change from 239 to 240
- `type_index_keys` grows by one entry
- `label_map.json` grows by one entry
- The hardcoded `GEOGRAPHIC_LABELS` array in `label_category_map.rs` must add `state_code` (and the assertion `test_total_is_239` must change to 240)
- The `GEOGRAPHIC_LABELS.len()` assertion (currently 24) must change to 25

**If wrong:** Training produces a model with n_classes=239 that cannot output state_code, or a model with n_classes=240 that crashes at inference because label_map is misaligned. The test suite will also fail on the hardcoded count assertions.

**Severity: HIGH** — This is the single biggest gap. The spec should explicitly call out n_classes=240 and list all downstream artifacts that need updating.

### A2: merged_dim calculation is correct
**Where:** ac-06 says "merged dim changes from 1006 to 1070."
**Problem:** The v12 config shows: char_hidden[1]=450, embed_hidden[1]=300, stats_hidden[1]=96, header_hidden[1]=96, valid_hidden[1]=64. That's 450+300+96+96+64 = 1006. The spec proposes valid_hidden changes from [128, 64] to [192, 128]. New merged_dim: 450+300+96+96+128 = 1070. This is correct **if** the other branch dimensions stay the same. But with valid_dim=240 (not 239), the first layer input changes too, though that doesn't affect merged_dim (only the final hidden dim matters for merge). So 1070 is correct for the merge dimension.
**Risk: LOW** — The calculation is correct, but should be documented explicitly.

### A3: Existing distilled data filtering mechanism exists
**Where:** ac-02 says "Original distilled CSV is not modified (filter produces a new file or applies at training time)."
**Problem:** The spec does not specify HOW the filtering happens. The current pipeline runs through `prepare_multibranch_data.py`, which loads distilled data from `sherlock_distilled.csv.gz`. There is an existing `--validate-labels` flag that does format-checking, but it validates against FineType's own inferences — it doesn't filter by type key or by value patterns as ac-02 requires.
**If wrong:** Implementer may modify the distilled CSV directly (violating the constraint), or may not know where in the pipeline to inject filtering.
**Severity: MEDIUM** — Should specify whether filtering is in the Python data prep script (new flag/config), in a standalone script, or as a pre-processing step.

### A4: v12 baseline is 204/227
**Where:** ac-09 says "Compare against v12 baseline (204/227)."
**Problem:** CLAUDE.md says the current default model (v11) scores 201/227. The v12 audit document says v12 also scored 204/227. The handover notes clarify that 204/227 is the correct v11 re-eval number. The spec should clarify that the baseline comparison target is the most recent eval result (204/227 from the April 16 re-eval), not the CLAUDE.md number (201/227).
**If wrong:** Target of 210/227 might be computed against wrong baseline — either +6 improvement (from 204) or +9 (from 201).
**Severity: LOW** — The target is absolute (210/227), so the baseline discrepancy doesn't affect the acceptance criterion. But it creates confusion.

### A5: Gradient norms are computable per-branch in Candle
**Where:** ac-07 proposes per-branch L2 gradient norms.
**Problem:** Candle's API provides `Tensor::grad()` after backward pass, but extracting per-branch gradient norms requires iterating over named parameters and grouping them by branch prefix (e.g., `char_l1`, `embed_l1`). The existing training loop (`multi_branch.rs:2416`) constructs `EpochMetrics` after the forward/backward pass but doesn't currently access any gradients. This is feasible in Candle but the spec should note that it requires parameter name introspection.
**If wrong:** Implementation is harder than expected, may require VarMap refactoring.
**Severity: LOW** — The existing gradient flow test at line 2745 shows gradients are already accessible in tests. The training loop just needs to extract them.

### A6: 50 epochs is enough for convergence
**Where:** Constraint says 50 epochs, interview notes v12 peaked at epoch 38.
**Problem:** The architecture is getting larger (validation branch doubles). Larger models sometimes need more epochs to converge. The spec has no early stopping criterion documented.
**If wrong:** Model may not converge in 50 epochs, or may overfit if convergence happens much earlier.
**Severity: LOW** — The exit condition says "50 epochs or early stopping," and the training loop already has EarlyStopping. This is fine.

---

## 2. Failure Mode Analysis

### ac-01 (state_code type): Could pass tests but fail in production

- **Pass in test, fail in production:** `cargo run -- check` validates taxonomy/generator alignment but does NOT validate that `label_category_map.rs` includes the new type, nor that `n_classes` is updated, nor that the multi-branch inference pipeline can output it. A model trained on 240 classes but loaded with an old `label_map.json` of 239 entries would silently mismap all predictions above the insertion index.
- **Missing:** No verification that `label_category_map.rs` is updated with `state_code` in `GEOGRAPHIC_LABELS`. No verification that test assertions (`test_total_is_239`) are updated.

### ac-02 (distilled data filtering): Could pass but leave contamination

- **Pass in test, fail in production:** The verification says "Filtered output has zero SSN rows, zero user_agent rows." But if the filter only runs on a test subset or the overnight script doesn't invoke the filter, the actual training data could still be contaminated.
- **Missing:** No verification that the overnight training script integrates the filter. The spec should require that the overnight script explicitly calls the filter (or uses the filtered file as input).

### ac-03 (validation patterns): Could pass but not improve model

- **Pass in test, fail in production:** Adding validation patterns to the taxonomy helps the `finetype validate` pipeline but only helps the model if the validation branch uses them at training time. The validation feature extractor runs validators at feature-extraction time (in `prepare_multibranch_data.py` or the Rust `ValidationFeatureExtractor`). The patterns will be picked up automatically. This is fine.
- **Edge case:** Latitude validation `[-90, 90]` will match most decimal_number values in that range, creating a high pass rate for latitude on non-latitude columns. The retrain brief already flags this as uncertain. The spec should acknowledge this risk more explicitly.

### ac-06 (architecture change): Inference code needs updating too

- **Critical:** The spec says "Update MultiBranchConfig default and v13 config.json." But `MultiBranchConfig` exists in TWO places:
  1. `crates/finetype-train/src/multi_branch.rs` (training side)
  2. `crates/finetype-model/src/multi_branch.rs` (inference side)
  
  The training config struct includes `valid_hidden` with `#[serde(default)]` so it deserializes from config.json. The inference side also reads config.json. The architecture change (valid_hidden [192, 128]) will be captured by config.json and deserialized by both sides automatically — no code change is needed to either struct. **However**, the `Default` impl in `finetype-model` still shows `valid_hidden: [0, 0]` (default for backward compat). The spec says "Update MultiBranchConfig default" — this could mean updating the default in the source code, which would break backward compat for models without valid_hidden in their config.
  
  **Clarification needed:** Does "update default" mean change the code default, or just ensure the v13 config.json has the right values? The safe answer is: only change config.json, leave the code default as `[0, 0]` for backward compatibility.

### ac-07 (gradient norms in EpochMetrics): Backwards compatibility

- **Critical:** `EpochMetrics` derives `Serialize, Deserialize`. Adding a new field `branch_gradient_norms: HashMap<String, f32>` without `#[serde(default)]` will break deserialization of ALL existing `results.json` files. Every tool, script, or analysis that reads old results.json will fail with a missing-field error.
- **The fix is simple:** Add `#[serde(default)]` to the new field. But the spec doesn't mention backwards compatibility at all.
- **Also:** The TUI renderer's `on_epoch_end()` will receive `EpochMetrics` with the new field. The TUI may or may not display it — this is fine since it's additive.

### ac-08 (training run): No rollback plan

- If training crashes at epoch 30 (Metal OOM, disk full, etc.), there's no specification of whether to restart from scratch or resume from a checkpoint. The training loop may or may not write periodic snapshots.

### ac-09 (eval run): Target may not be achievable

- The retrain brief's conservative estimate is 210-212/227, and that's if all P1+P2+P3 fixes work. P4 (architecture) is flagged as "higher-risk." If v13 scores 208/227, the spec's exit condition says "≥210/227 or findings documented." This is well-handled.

---

## 3. Test Adequacy

```
| AC    | Verification                                  | Adequate? | Gap                                                    |
|-------|-----------------------------------------------|-----------|--------------------------------------------------------|
| ac-01 | cargo check + generate                        | PARTIAL   | Missing: label_category_map.rs update, n_classes=240,  |
|       |                                               |           | valid_dim=240, test assertion updates                   |
| ac-02 | Filter script exists + counts verified         | PARTIAL   | Missing: integration with overnight script              |
| ac-03 | cargo check + cargo test + discrimination test | GOOD      | Minor: latitude false-positive rate not bounded         |
| ac-04 | Count query on training data                   | GOOD      |                                                        |
| ac-05 | Hard negatives present + header/value checks   | GOOD      |                                                        |
| ac-06 | Config shows valid_hidden + cargo test passes  | PARTIAL   | Missing: inference-side compatibility check             |
| ac-07 | results.json has gradient norms                | PARTIAL   | Missing: backward compat (#[serde(default)])            |
| ac-08 | Model directory exists with all files          | GOOD      |                                                        |
| ac-09 | Eval report exists + accuracy recorded         | GOOD      |                                                        |
```

---

## 4. Gap Analysis

### G1: n_classes 239 to 240 cascade (HIGH)
Adding state_code creates a 240th type. The spec never acknowledges this. Affected artifacts:
- `MultiBranchConfig.n_classes`: 239 → 240
- `MultiBranchConfig.valid_dim`: 239 → 240
- `label_map.json`: 239 → 240 entries
- `type_index_keys` in config.json: 239 → 240 entries
- `label_category_map.rs`: `GEOGRAPHIC_LABELS` needs `state_code`, `test_total_is_239` → `test_total_is_240`, `GEOGRAPHIC_LABELS.len()` assertion 24 → 25
- `CLAUDE.md`: "239 definitions" → "240 definitions"
- Eval: `schema_mapping.yaml` may need state_code entries if any eval dataset contains state codes
- DuckDB extension: No code change needed (reads config.json dynamically)

### G2: No rollback plan
If v13 regresses below v12 (204/227), the spec says "models/default symlink stays on sherlock-v11" which is correct. But there's no explicit statement about what happens to the v13 artifacts — are they kept for analysis or deleted?

### G3: Missing overnight script specification
The spec says "overnight training run on M1 Mac with Metal" but doesn't specify whether to modify the existing `overnight_v12_retraining.sh` or create a new `overnight_v13_retraining.sh`. Given that the data prep, filtering, and architecture all change, a new script is almost certainly needed.

### G4: Hard-negative mining implementation unspecified (ac-05)
The spec says "Generate synthetic decimal_number examples in [-90, 90] with non-geographic headers." But it doesn't specify:
- Where these examples are generated (in `prepare_multibranch_data.py`? In a new script? In the Rust generator?)
- How many hard negatives to add per decimal_number
- Whether they supplement or replace existing decimal_number synthetic data

### G5: Distilled cap timing relative to filtering (ac-02 vs ac-04)
ac-02 filters bad rows (SSN: 3→0, user_agent: 4→0, phone: partial, postal_code: 34→~4).
ac-04 caps distilled at 600/type.
The spec says the cap is "applied before the 70/30 blend ratio" but doesn't say whether filtering happens before or after the cap. If filtering happens after capping, you might cap at 600 then filter down further, which is fine. If capping happens before filtering, you cap contaminated data at 600 then remove bad rows, also fine. But the order should be explicit: **filter first, then cap**.

### G6: No error handling for gradient norm computation (ac-07)
If gradient computation fails or returns NaN for a branch, what should the training loop do? Log a warning and continue? Skip the field? The spec should at minimum say `branch_gradient_norms` is optional or has a defined fallback.

---

## 5. Constraint Check

### Contradiction: merged_dim 1006 vs 1070
The spec says "merged dim 1006→1070" in constraints and ac-06. This is internally consistent. Verified: 450+300+96+96+64=1006 (current) vs 450+300+96+96+128=1070 (proposed). Correct.

### Realistic: 50 epochs overnight on M1
v12 ran 40 epochs. Each epoch on M1 Pro with Metal takes ~90-120 seconds for 1200 columns/type (roughly 239*1200 = 287k training samples). 50 epochs * ~100s = ~5000s = ~83 minutes. This is very feasible overnight.

### Realistic: Target 210/227 (92.5%)
The retrain brief estimates P1+P2 alone should fix 6-8 items, P3 adds 2-3. Starting from 204/227, that's 210-215/227. This is achievable but depends on all fixes working as expected. The spec's exit condition allows documenting the gap if the target isn't met. Reasonable.

### Missing constraint: CLAUDE.md and test assertion updates
The constraint list doesn't mention updating CLAUDE.md ("239 definitions"), `label_category_map.rs` assertions, or the geography domain count. These are all implied by ac-01 but should be explicit.

---

## Summary of Required Changes

### Must fix before implementation:

1. **[HIGH] ac-01: Document n_classes cascade.** Explicitly state that adding state_code changes n_classes from 239 to 240. List all artifacts that need updating: `label_category_map.rs` (GEOGRAPHIC_LABELS + test assertions), `valid_dim` in config, n_classes in config, CLAUDE.md type count. Add a verification step: `cargo test -p finetype-model` must pass (this will catch the assertion failures).

2. **[HIGH] ac-07: Require `#[serde(default)]` on `branch_gradient_norms`.** The new field MUST have `#[serde(default)]` to preserve backward compatibility with existing `results.json` files. State this explicitly in the AC description.

3. **[MEDIUM] ac-02: Specify filtering mechanism.** State whether filtering is a new flag in `prepare_multibranch_data.py`, a standalone pre-processing script, or a configuration file. Also specify that filtering runs before the distilled cap (ac-04).

4. **[MEDIUM] ac-06: Clarify "update MultiBranchConfig default."** If this means changing the Rust code default, flag the backward compat risk. Recommendation: do NOT change the code default (leave `valid_hidden: [0, 0]` for backward compat). Only set the values in v13's config.json.

### Should fix:

5. **[LOW] ac-05: Specify hard-negative count and mechanism.** How many hard-negative decimal_number examples? Where in the pipeline?

6. **[LOW] Deliverables: Add `label_category_map.rs` to the deliverables list.** It's a code change required by ac-01 that isn't listed.

7. **[LOW] Add `overnight_v13_retraining.sh` to deliverables.** The overnight script is an obvious deliverable.

---

## Verdict: REQUEST_CHANGES

The spec is solid in intent and scope. The retrain brief provides excellent evidence for each priority tier. The main gaps are around the n_classes cascade from adding state_code (which touches 6+ files beyond the two listed in deliverables) and the backward compatibility of the EpochMetrics change. These are not architectural concerns — they're completeness issues that would cause implementation confusion or test failures if not addressed upfront. Fix the 4 "must fix" items and this is ready to implement.
