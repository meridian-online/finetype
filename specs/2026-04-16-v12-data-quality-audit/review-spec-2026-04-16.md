# Spec Review

**Date:** 2026-04-16
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-04-16-v12-data-quality-audit/spec.yaml
**Verdict:** BLOCK

## Findings

### [CRITICAL] Spec premises contradict the actual eval data

**Category:** Data integrity
**Description:** The spec and interview claim v12 scored 204/227 (+3 over v11's 201/227) with 23 misclassifications, 11 fixes, 8 regressions, and 9 of 11 persistent errors increasing in confidence. The actual v12 eval report at `/tmp/eval-v12/models/sherlock-v12/eval/report.md` shows 201/227 (26 misclassifications). Cross-referencing with the in-repo v11 eval at `models/sherlock-v11/eval/profile_results.json` (203/227, 24 misclassifications), the real numbers are:

```
| Metric                          | Spec claims | Actual data |
|---------------------------------|-------------|-------------|
| v11 score                       | 201/227     | 203/227     |
| v12 score                       | 204/227     | 201/227     |
| Direction                       | +3 gain     | -2 regression|
| v12 misclassifications          | 23          | 26          |
| Fixed by v12                    | 11          | 13          |
| Regressed in v12                | 8           | 15          |
| Persistent errors               | 11          | 11          |
| Persistent errors conf UP       | 9           | 3           |
| Persistent errors conf DOWN     | (implied 2) | 8           |
```

The confidence direction claim is inverted. Only 3 persistent errors increased confidence in v12 (year 0.83->1.00, depthError 0.61->0.97, user_agent 0.37->1.00). The other 8 decreased. This means the validation branch is not systematically "reinforcing wrong patterns" as the spec assumes -- it is reducing confidence on most persistent errors.

**Evidence:** `python3` cross-comparison of `models/sherlock-v11/eval/profile_results.json` (203/227) and `/tmp/eval-v12/models/sherlock-v12/eval/profile_results.json` (201/227). The v12 model config confirms 5-branch architecture (`valid_dim: 239`).

**Recommendation:** Re-derive all premise numbers from the actual eval JSON before any audit work begins. The audit scope changes from "23 items" to "26 items." The narrative changes from "v12 improved but has churn" to "v12 regressed overall -- the validation branch may be doing more harm than good." This changes the framing of ac-04 and ac-07 significantly.

---

### [CRITICAL] ac-03 scopes training data inspection to "8 regressed types" but the regression count is wrong

**Category:** Scope error
**Description:** ac-03 says "Training data inspection for the 8 regressed types. For each regression pair..." but the actual regression count is 15, not 8. This halves the training data inspection scope. Auditing only 8 of 15 regressions would miss patterns in the other 7.

Additionally, the ac-03 wording "regression pair (e.g., phone_number vs ssn)" references a v11 confusion pattern (phone->ssn) that is actually *fixed* in v12, not a regression. The v12 regressions include entirely different patterns (country->country_code, dmy_dash->iso, sepal_length->version, etc.) that are not mentioned anywhere in the spec.

**Evidence:** The 15 actual v12 regressions include:
- 3 new date format confusions (dmy_dash, iso_week, mdy_dash all predicted as `iso`)
- 2 country/country_code confusions (new pattern)
- 2 decimal_number->version confusions (iris.sepal_length, iris.sepal_width)
- 2 new subtype confusions (email_display->email, phone_e164->phone_number)
- 1 data_uri->url, 1 pe_ratio->latitude, 1 horizontalError->dmy_short_dot
- 1 isin->alphanumeric_id, 1 order_id->isbn, 1 user_agent->plain_text

**Recommendation:** Fix the regression count to 15 and update ac-03 to cover all regressions. Consider whether "regression pairs" is still the right framing -- some regressions (e.g., country->country_code) may be GT issues rather than training data issues.

---

### [MAJOR] ac-02 requires exactly one root cause per item, but many misclassifications have compound causes

**Category:** Methodology
**Description:** ac-02 says "Every item has exactly one root cause." In the prior eval-audit-v2, several cases were genuinely multi-causal. For example, decimal_number being confused with latitude involves both (a) training data overlap in the value range and (b) insufficient header utilisation. Forcing a single root cause risks losing the compound signal needed for the retrain brief.

The root cause categories also have a gap: `gt_error` is listed but the eval-audit-v2 identified several "DEBATABLE" cases that are neither wrong GT nor model error -- they are hierarchy/subsumption issues (e.g., email_display vs email, phone_e164 vs phone_number, country vs country_code). These need a category like `hierarchy_gap` or `subsumption` that is distinct from `gt_error`.

**Evidence:** The v12 regressions include 5 cases where the prediction is the parent type of the expected type (email->email_display, phone_number->phone_e164, url->data_uri, country->country_code x2). These are systematically different from "model_error" -- the model is correct at the parent level but misses the subtype distinction.

**Recommendation:** Allow a primary and secondary root cause. Add a `hierarchy_subsumption` root cause category for parent-type predictions. This distinction matters for the retrain brief: hierarchy issues need subtype-discriminating training data, not the same fixes as domain-level confusion.

---

### [MAJOR] ac-04 confidence analysis premise is backwards

**Category:** Methodology
**Description:** ac-04 says to analyse "the 9 persistent errors that increased in confidence" to determine "whether the validation branch is reinforcing or neutral toward the wrong prediction." Only 3 persistent errors increased confidence, not 9. Designing a validation-branch investigation around 3 items is less useful than designing it around all 11 persistent errors (3 up, 8 down) to understand the full picture.

The framing "is the validation branch reinforcing wrong patterns" assumed 9/11 trending up. With only 3/11 trending up and 8/11 trending down, the more interesting question is: "why did the validation branch fix the confidence signal for most persistent errors but make 3 dramatically worse (year 0.83->1.00, depthError 0.61->0.97, user_agent 0.37->1.00)?"

**Evidence:** Cross-reference of persistent error confidence changes shows only year, depthError, and user_agent increased. All three hit confidence 0.97+ in v12, suggesting the validation features are providing strong but wrong signals for these specific types.

**Recommendation:** Reframe ac-04 to analyse all 11 persistent errors. The question becomes "what makes the 3 that increased confidence different from the 8 that decreased?" This is more actionable for the retrain brief.

---

### [MAJOR] Circular reasoning risk in GT corrections (ac-05) lacks safeguard

**Category:** Methodology
**Description:** The spec allows correcting GT to match model predictions (ac-05) and then computing an adjusted score (ac-06). This creates a circular reasoning risk: the auditor could rationalise GT corrections to inflate the v12 score, especially under the narrative that "v12 improved" (which the data does not actually support).

Decision 0037 established that GT can be updated when evidence contradicts, but it also explicitly notes the consequence: "profile eval regressions require investigation to confirm the fix is genuinely correct before updating expectations." The spec's ac-05 verification is "Corrected labels match the evidence from ac-01" -- but ac-01's evidence is collected by the same auditor, creating a single-rater problem.

The eval-audit-v2 (prior art) had a rigorous per-case analysis with explicit value samples and format analysis. The spec should require the same rigour but does not specify the evidence standard.

**Evidence:** Decision 0037 explicitly calls out the "bad consequence" of needing investigation before GT changes. The spec's verification criteria are self-referential (ac-05 verified by ac-01, which is verified by "every row has a verdict and evidence" with no external standard for what counts as sufficient evidence).

**Recommendation:** Add an evidence standard for GT corrections:
1. Sample at least 10 values from the column and show them in the audit table.
2. For each GT correction, state the alternative interpretation and why it is weaker.
3. Require that GT corrections can only flip the ground truth *toward* a more general type (subsumption) or *toward* a format-validated specific type, never simply toward the model's prediction without independent validation.

---

### [MAJOR] No baseline comparison methodology for ac-06

**Category:** Completeness
**Description:** ac-06 says "Re-run eval or compute adjusted score from the audit table." These two methods can produce different results. "Compute from the audit table" means manually subtracting GT-error items from the miss count, which does not account for cascading effects (e.g., a GT correction might change which interchangeability rules match, affecting other columns' scoring). "Re-run eval" is deterministic but requires committing the GT changes first.

The spec does not say which method to use or how to reconcile differences. If the adjusted score is computed by hand, it should state the formula explicitly.

**Evidence:** The eval framework has interchangeability rules in `matching.rs` that could change scoring for related types. A GT correction that adds an alternative accepted label could affect scoring for other columns of the same type.

**Recommendation:** Require re-running the eval after committing GT corrections (ac-05), not hand-computing. This is the only way to catch cascading effects. State that the re-run uses the same model binary and the updated `schema_mapping.yaml`.

---

### [MINOR] ac-03 sample sizes may be insufficient for rare types

**Category:** Methodology
**Description:** ac-03 requires "sample 20 distilled examples and 20 synthetic examples from each type." For types with low distilled coverage, 20 examples may not exist. The spec does not say what to do if fewer than 20 distilled examples are available for a type -- report the gap? Adjust the sample size? Flag the type as data-starved?

**Evidence:** Prior collisions audit (specs/2026-03-26) noted that some types have very few distilled training examples. The distillation pipeline processed 5,364 columns but coverage per type is uneven.

**Recommendation:** Add a fallback: "If fewer than 20 distilled examples exist for a type, report the actual count and flag the type as `data_gap` in the root cause analysis." This connects naturally to the ac-02 `data_gap` category.

---

### [MINOR] Constraint "Prior audit findings are starting context, not repeated work" is ambiguous

**Category:** Clarity
**Description:** The constraint says the prior audits (eval-audit-v2 at 34 items, collisions.md at 23 pairs) are "starting context, not repeated work." But 11 of the 26 v12 misclassifications are persistent from v11, meaning they were already audited in eval-audit-v2. Does "not repeated work" mean (a) skip the persistent items entirely, (b) reference the prior verdicts without re-investigating, or (c) re-investigate but start from the prior conclusion?

Option (a) would leave 11 items unaudited, violating the "all 23 [actually 26] audited equally" constraint. Option (b) could miss changes in root cause (e.g., a v11 model_error might become a training_collision in v12 due to new training data). Option (c) is the most rigorous but contradicts "not repeated work."

**Evidence:** The constraint "All 23 misclassifications audited with equal rigour -- no priority triage" conflicts with "prior audit findings are starting context, not repeated work" for the 11 persistent items.

**Recommendation:** Clarify: "For persistent errors, reference the prior audit verdict and note whether the root cause has changed. A full re-investigation is only needed if the v12 prediction or confidence differs materially from v11." This resolves the contradiction.

---

### [MINOR] Deliverable paths assume the spec narrative is correct

**Category:** Naming
**Description:** The deliverable `confidence-analysis.md` is scoped to "persistent errors" but ac-04's premise (9 items increasing confidence) is wrong. The analysis should cover all 11 persistent errors and possibly the 15 regressions too. The file name and scope should be updated after the premise correction.

**Recommendation:** Rename to something neutral like `validation-branch-analysis.md` that does not embed assumptions about direction.

---

### [INFO] The eval-audit-v2 was done on v4-sibling (193/227), not v11

**Category:** Context
**Description:** The prior art eval-audit-v2 audited 34 misclassifications at 193/227 on the v4-sibling model. The v11 model (203/227) and v12 model (201/227) have a different misclassification set. Only 11 items persist from v11 to v12, and fewer persist from v4 to v12. The prior audit's verdicts (WRONG/DEBATABLE) need rechecking for items that are still present -- the model may have changed its prediction even for persistent misses.

**Evidence:** v4-sibling eval-audit-v2 case 9 (ecommerce_orders.phone: ssn->phone_number) is fixed in v12. Case 22 (method: categorical->http_method) is fixed in v12. Several "DEBATABLE" cases from v4 may no longer be misclassifications in v12.

**Recommendation:** Build a 3-way comparison table (v4 vs v11 vs v12) for the persistent items to track how predictions evolved across model versions.

---

### [INFO] The v12 model architecture (5-branch with validation features) changes the investigation methodology

**Category:** Context
**Description:** The v12 model config shows `valid_dim: 239` and `valid_hidden: [128, 64]`, confirming a 5th branch (validation features) not present in v11 (4-branch). ac-04 correctly identifies this as worth investigating, but the spec does not provide methodology for extracting per-sample validation features. How will the implementer extract the 239-dim validation vector? Is there a CLI command, a debug mode, or does it require code changes?

**Evidence:** The v12 config.json has the validation branch architecture. The codebase has multi-branch inference in `column.rs` but the extraction of intermediate features may not be exposed in the CLI.

**Recommendation:** Specify how to extract validation features. If no tooling exists, add a sub-task to ac-04: "Build or use existing debug tooling to extract per-column validation feature vectors."

---

## Assumption Audit

```
| # | Assumption                                          | Validated by AC? | Risk if wrong                                              |
|---|-----------------------------------------------------|------------------|------------------------------------------------------------|
| 1 | v12 scored 204/227 with 23 misclassifications       | No (premise)     | CRITICAL: Entire audit scope is wrong (actual: 26 items)   |
| 2 | 11 fixes, 8 regressions in v12                      | No (premise)     | CRITICAL: Regression scope for ac-03 is halved (actual: 15)|
| 3 | 9 of 11 persistent errors increased confidence       | ac-04            | MAJOR: Confidence analysis question is backwards           |
| 4 | Validation branch reinforces wrong patterns           | ac-04            | MAJOR: Only 3/11 went up; 8/11 went down                  |
| 5 | Training data inspection can explain regressions      | ac-03            | LOW: Some regressions may be architecture-level            |
| 6 | 20 distilled examples exist per regressed type        | No               | MINOR: Some types may be data-starved                      |
| 7 | GT corrections are unambiguous when evidence supports | ac-05            | MAJOR: Circular reasoning without external validation      |
| 8 | Adjusted score can be computed from audit table       | ac-06            | MINOR: Misses cascading interchangeability effects         |
| 9 | Prior audit verdicts still apply to persistent items  | No               | MINOR: Model changed predictions even for persistent items |
```

## Honest Assessment

This spec has a solid structure and follows good investigative principles -- upstream-first root cause analysis, per-item evidence, actionable output. The orbit workflow produced clean acceptance criteria with logical flow from audit (ac-01/02) through investigation (ac-03/04) to action (ac-05/06/07). The evaluation principles are well-chosen. However, the spec is built on premises that are factually wrong in every major dimension: the v12 score, the regression count, the fix count, and the confidence direction. These are not edge-case discrepancies -- the spec says v12 improved by 3 when it actually regressed by 2, and says 9/11 persistent errors got more confident when only 3/11 did. Implementing this spec as-written would audit 23 items when 26 exist, inspect training data for 8 regressions when 15 exist, and investigate a "validation branch reinforcement" hypothesis that the data does not support. The spec structure is sound and reusable after correcting the premises. I recommend blocking until the factual basis is re-derived from the eval JSONs, then reissuing with corrected numbers and adjusted framing.
