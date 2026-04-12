# Spec Review: Accuracy Gap Retraining

**Reviewer:** Nightingale (fresh context)
**Date:** 2026-04-12
**Spec:** `specs/2026-04-12-accuracy-gap-retraining/spec.yaml`
**Verdict:** REQUEST_CHANGES

---

## 1. Assumption Audit

### A1: Hyperparameters from autoresearch transfer to ReLU+BN (HIGH RISK)

The spec prescribes `lr=1e-3, weight_decay=0.01` citing "Autoresearch: weight_decay=0.01, lr=1e-3 validated on base architecture." This is misleading. The autoresearch spec (`specs/2026-04-11-candle-autoresearch-port/spec.yaml` ac-09, ac-11) validated these hyperparameters exclusively on GELU+LN architecture. The actual baseline model (v4-sibling, which achieved 193/227) was trained with `lr=1e-4, weight_decay=1e-4` -- a 10x difference in both parameters.

**Risk:** Applying lr=1e-3 to ReLU+BN at production-scale dimensions may cause training instability or divergence. ReLU+BN and GELU+LN have different loss landscape characteristics; hyperparameters do not transfer automatically between activation/normalization regimes.

**Not validated by any AC.** AC-07 checks `val_accuracy >= 84%` which would catch catastrophic failure, but a subtler issue (converges to a worse local minimum at 84.5% vs 90% with correct hyperparameters) would pass the gate and produce an inferior model.

**Recommendation:** Either (a) use the hyperparameters that v4-sibling actually trained with (`lr=1e-4, weight_decay=1e-4`) as the safe default, or (b) explicitly state that lr=1e-3 is experimental for ReLU+BN and add a constraint/exit condition: "If val_accuracy plateaus below v4-sibling baseline (90%), retry with lr=1e-4."

### A2: Production-scale dimensions are correct (MEDIUM RISK)

The constraint says `[450,450]/[300,300]/[192,96]/[750,750]` but this lists only 4 dimension groups. The actual model config has 5 groups: char_hidden `[450,450]`, embed_hidden `[300,300]`, stats_hidden `[192,96]`, header_hidden `[192,96]`, merge_hidden `[750,750]`. The omitted header_hidden is ambiguous -- is it `[192,96]` (matching v5-scaled config) or `[128,64]` (matching v4-sibling)?

**Impact:** If the implementer reads the constraint literally and constructs a config missing header_hidden, serde defaults will apply silently. The spec should reference the config file by name (`models/sherlock-v5-scaled-config.json`) rather than enumerating dimensions inline.

### A3: 70/30 distillation:synthetic ratio is achievable (LOW RISK)

The existing `prepare_multibranch_data.py` defaults to 50/50. The `--ratio-distilled` flag exists and accepts any float 0.0-1.0, so changing to 0.7 is mechanically trivial. However, the v3 format's ratio application works at the table-group level (line ~1514: "ratio_distilled of total groups should be distilled"), not per-type. Types with low distilled coverage may still be synthetic-dominated regardless of the global ratio.

**Not validated by any AC.** AC-03 says "Record total samples, type coverage, and mix ratio" but does not set a per-type minimum distilled coverage threshold. The 74.6% Sherlock coverage means ~60 of 239 types have zero distilled data and will be 100% synthetic regardless of the 70/30 target.

### A4: 102k Sherlock annotations are high quality (MEDIUM RISK)

Interview open question: "Whether the 102k Sherlock annotations need re-filtering (74.6% coverage -- some annotations may be low quality)." This question was surfaced but not answered. The spec proceeds to use this data as-is.

**Risk:** The distillation was performed by LLMs (per CLAUDE.md memory). Decision 0038 establishes "LLMs for parsing, programmatic checks for validation" as a principle. Using LLM-generated labels as training data is different from using them for parsing, but systematic labelling errors will be amplified by 70/30 weighting.

**Mitigation exists but is incomplete:** AC-01 audits the 34 eval failures, not the 102k training labels. A type-level quality spot-check of the distillation data is not in scope.

### A5: The eval set is stable at 227 columns (LOW RISK)

AC-02 may change the eval set by adding interchangeability rules, which could increase the baseline above 193/227 without any model change. The target of 205/227 may then need recalibration.

**Not a blocker** -- the spec handles this by saying "Baseline recalibrated" in ac-02 verification. But the target number in ac-08 is hardcoded to 205. If the audit finds, say, 8 DEBATABLE labels, the baseline becomes ~201/227 and reaching 205 requires only 4 genuine improvements. The threshold becomes trivially easy.

### A6: Model naming assumes v11 (LOW RISK)

Deliverables reference `models/sherlock-v11/` and `scripts/overnight_v11_retraining.sh`. Previous models are v1 through v10. v10 was the GELU+LN experiment. The v11 name is fine, but should be explicitly stated as a constraint so the implementer does not accidentally reuse v10's directory.

---

## 2. Failure Mode Analysis

### AC-01 (Audit): Could pass but mislead

The audit classifies each of the 34 misclassifications as WRONG/DEBATABLE/AMBIGUOUS. Risk: an implementer unfamiliar with the domain might classify genuinely wrong model predictions as DEBATABLE (especially the "type-specific single errors" cluster of 12 items). This inflates the baseline without fixing real problems.

**Production risk:** Overly generous DEBATABLE classification leads to relaxed interchangeability rules that mask real accuracy problems downstream.

### AC-03/AC-04/AC-05 (Data preparation): FTMB format correctness

The v10 script uses Python (`prepare_multibranch_data.py`) with `--ratio-distilled 0.5`. The new spec needs 0.7. The script exists and supports this flag. However:

- The v10 script hardcodes `--workers 8`. Mac Metal training typically runs on a machine that may not have resources for 8 parallel feature extraction workers alongside compilation.
- Header validation (AC-04) spot-checks "first 5 groups" -- if the FTMB has thousands of groups, this is statistically weak. Zero-header groups could appear later.

### AC-06/AC-07 (Training): val_accuracy gate may be too loose

The spec uses `val_accuracy >= 84%` as the pass threshold. The v4-sibling baseline achieved 89.99% val_accuracy. An 84% gate allows a 6-point val_accuracy regression to pass. The relationship between val_accuracy and profile eval accuracy is non-linear (the Sharpen layer can amplify or dampen model improvements). A model with 84% val_accuracy might score anywhere from 170-200 on profile eval.

**Recommendation:** Tighten the pass gate to `>= 88%` (within 2 points of v4-sibling) and keep `< 84%` as the abort threshold.

### AC-08 (Profile eval): Environment sensitivity

Profile eval results depend on:
- The Sharpen rules (F1-F6, R1-R19) which are hardcoded in `column.rs`
- Model2Vec model at `models/model2vec/`
- Sibling-context model at `models/sibling-context/`

If any of these change between the v4-sibling baseline measurement and the v11 measurement, the comparison is confounded. The spec does not explicitly freeze Sharpen rules or auxiliary models.

### AC-11 (HuggingFace publish): No rollback plan

If the model is published and the DuckDB extension starts downloading it, there is no way to revert to v4-sibling except by re-publishing. The spec should include a rollback procedure (e.g., keep v4-sibling as a tagged release, publish v11 under a new tag, update the "latest" pointer only after a soak period).

---

## 3. Test Adequacy

```
| AC   | Verification Method                                    | Adequate? | Notes                                                |
|------|--------------------------------------------------------|-----------|------------------------------------------------------|
| ac-01| "Audit document exists with verdict for all 34 cases"  | WEAK      | No quality check on verdicts themselves               |
| ac-02| "make eval-report reflects corrected ground truth"     | OK        | Deterministic, repeatable                             |
| ac-03| "FTMB file exists. Record total samples, type coverage"| WEAK      | Existence check, no quality gate on mix ratio          |
| ac-04| "Binary FTMB validation script confirms non-zero"      | OK        | Spot-check is reasonable for header validation         |
| ac-05| "FTMB prep log shows sibling-context enrichment"       | WEAK      | Log existence != correctness. Could log "applied" with a bug |
| ac-06| "Training completes. results.json written."             | OK        | Binary pass/fail                                       |
| ac-07| "results.json best_val_accuracy field"                  | OK*       | See A1 -- threshold may be too loose                   |
| ac-08| "make eval-report. Compare against 193/227 baseline."  | OK        | Deterministic comparison                               |
| ac-09| "make eval-report actionability section."               | OK        | Deterministic                                          |
| ac-10| "Delta analysis in progress.md"                        | WEAK      | Document existence, no quality gate                    |
| ac-11| "HuggingFace model card updated."                      | OK        | Verifiable                                             |
| ac-12| "CLAUDE.md mentions new model name."                   | OK        | Verifiable                                             |
```

### Strengthening recommendations:
- **AC-03:** Add a gate: "Per-type distilled coverage report. At least 150/239 types have >= 1 distilled example."
- **AC-05:** Validate by checking FTMB header feature variance across groups, not just the prep log message.

---

## 4. Gap Analysis

### Missing: Config file for v11

The spec says ReLU+BN at production-scale dimensions, but does not specify which config file to use. The implementer must either create a new config or use `sherlock-v5-scaled-config.json` (which exists and is correct). This should be an explicit deliverable.

### Missing: Epoch count

Interview open question: "Exact number of training epochs (v4-sibling used 30, autoresearch sweet spot was ~27-30)." The spec prescribes `patience=10` but no explicit epoch cap. The v10 script defaulted to 30 epochs. If the higher learning rate causes slower convergence, 30 may be insufficient.

### Missing: stats_dim handling

The v4-sibling config has `stats_dim: 27`, but CLAUDE.md says "36-dim deterministic feature extractor." If the feature extractor has been upgraded to 36 dimensions since v4-sibling was trained, the new FTMB will have stats_dim=36 and the model config needs to match. The spec does not address this.

### Missing: n_classes handling

Taxonomy has 239 types, config has `n_classes: 250`. The spec says "237 types" for synthetic data. These three numbers are all different. The spec should clarify: is n_classes 250 (padded), 239 (current taxonomy), or 237 (something else)?

### Missing: Rollback procedure for HuggingFace publish

If the published model causes regressions discovered after publication (e.g., on real-world data not covered by the 227-column eval set), how does the team revert?

### Missing: Training time estimate

Mac Metal training at production-scale dimensions with a 70/30 dataset (larger than the 50/50 used for v4-sibling) will take longer. No time estimate is provided. This matters for scheduling overnight runs.

---

## 5. Constraint Check

### Contradictions found:

1. **Hyperparameter mismatch (BLOCKING):** Constraint says `weight_decay=0.01, lr=1e-3`. These were validated on GELU+LN only. The constraint also says "ReLU+BN architecture." These two constraints have never been jointly validated. The v4-sibling baseline (ReLU+BN, 193/227) used `lr=1e-4, weight_decay=1e-4`.

2. **Dimension shorthand is incomplete:** Constraint says `[450,450]/[300,300]/[192,96]/[750,750]` -- 4 groups. Actual config requires 5 groups (header_hidden is missing from the shorthand).

### Constraints that are realistic:

- Mac Metal training only: reasonable, all prior training has been on Mac Metal.
- 70/30 distillation:synthetic mix: mechanically achievable via `--ratio-distilled 0.7`.
- No actionability regression: AC-09 checks this with a 0.4% tolerance (96.5% vs 96.9%).
- Backward compatible via serde defaults: already proven by v10 GELU+LN experiment.

---

## 6. Verdict

**REQUEST_CHANGES** -- one blocking issue and several improvements needed before implementation.

### Blocking

1. **Resolve hyperparameter mismatch.** The spec must decide: use v4-sibling-proven hyperparameters (`lr=1e-4, weight_decay=1e-4`) or the autoresearch hyperparameters (`lr=1e-3, weight_decay=0.01`). If the latter, acknowledge this is untested on ReLU+BN and add an exit condition for convergence issues. Do not present autoresearch results as applying to an architecture they were not tested on.

### Recommended

2. **Reference config file by name** instead of listing dimensions inline. Either `sherlock-v5-scaled-config.json` (existing, correct) or a new `sherlock-v11-config.json` with explicit fields. Add to deliverables.

3. **Tighten AC-07 val_accuracy gate** from 84% to 88%. Keep the 80% abort threshold.

4. **Add explicit epoch cap** (e.g., 40 epochs with patience=10, matching prior training runs).

5. **Add rollback procedure** for AC-11: "If post-publish regressions are found, re-publish v4-sibling as default. v11 remains available as a tagged release."

6. **Clarify n_classes and type count discrepancy** (237 vs 239 vs 250).

### Nice to have

7. **Strengthen AC-03** with per-type distilled coverage report.
8. **Strengthen AC-05** with header feature variance check in addition to log message.
9. **Add training time estimate** to help schedule the overnight run.
