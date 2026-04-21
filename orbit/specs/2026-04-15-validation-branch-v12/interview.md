# Design: Validation Branch for v12 Retraining

**Date:** 2026-04-15
**Interviewer:** Nightingale
**Card:** orbit/cards/0002-semantic-type-detection.yaml

---

## Context

Card: *Semantic type detection* — 4 scenarios, goal: 215+/227 (~95%) via retraining for remaining 26 model-level misclassifications.

Prior specs: 7 — Sherlock architecture (Mar 21) → multi-branch integration (Mar 25) → data quality audit expanding eval to 227 columns (Mar 26) → RunPod autoresearch finding GELU+LN (Apr 9) → Candle port rejecting GELU+LN via decision 0046 (Apr 11) → v11 retraining reaching 203/227 (Apr 12) → Sharpen header bugfixes reaching 201/227 ceiling (Apr 12).

Gap: 26 misclassifications remain, all model-level. 6 of top 8 are same-category sibling confusion at 1.00 confidence (country↔country_code, email↔email_display, phone↔phone_e164, url↔data_uri, compact_ym↔year). Sharpen tuning (hints/thresholds) confirmed exhausted per decision 0038. Actionability at 96.7% is caused by the same mispredictions, not taxonomy format_string gaps.

## Q&A

### Q1: Approach for same-category sibling confusion
**Q:** Six of the top 8 failures are same-category sibling confusion at 1.00 confidence. The values are nearly identical — only the header or structural pattern distinguishes them. How should we handle these?
**A:** The Sharpen layer could solve these cheaply — countries should have an enum, e164 already has a JSON Schema pattern. But the more principled approach: run validation on each value against type schemas, return boolean pass/fail, and feed the results into the model as a new feature branch. Existing JSON Schema infrastructure (jsonschema 0.42, CompiledValidator, microsecond-level validation) makes this cheap.

### Q2: Pipeline placement — where validation results are consumed
**Q:** Where in the pipeline should validation results be consumed?
**A:** New model feature branch (5th branch in multi-branch). The model learns to weight validation signal alongside char/embed/stats/header. Requires v12 retraining.

### Q3: Country code enum scope
**Q:** Should this spec also add the ISO 3166 enum to country_code's validation?
**A:** Yes — add ISO 3166-1 alpha-2 enum now. Makes the validation feature maximally discriminating for the country↔country_code pair.

### Q4: Validation feature dimensions
**Q:** Full 239-dim (one pass rate per type) vs category-level aggregation?
**A:** Full 239-dim. Consistent with other branch sizes (char: 960, embed: 512). Actually the smallest branch. Model's linear layer learns which matter.

### Q5: Training data computation
**Q:** Should validation features be computed on-the-fly during training or pre-computed into FTMB?
**A:** Pre-compute into FTMB. Requires FTMB v4 format but avoids repeated computation during training. One-time cost during data generation.

### Q6: Numeric range validation for latitude/longitude
**Q:** The validation branch can't reach latitude↔decimal_number because latitude has no schema pattern. Should we add numeric range constraints?
**A:** Yes — add range constraints now. Latitude: -90 to +90, Longitude: -180 to +180. Requires parsing string to float before checking range, adding complexity to the validation pipeline, but closes the one pair that pure pattern validation can't reach.

### Q7: Spec scope
**Q:** What's the target scope?
**A:** Full v12: validation branch architecture + FTMB v4 format + country_code enum + range constraints + retraining + eval. One spec.

### Q8: Eval target
**Q:** What's the eval target for v12?
**A:** 215/227 (95%). The card's stated target. 14 more correct out of 26 remaining.

---

## Summary

### Goal
Add a 5th "validation" branch to the multi-branch model that provides JSON Schema pass rates per type as input features. Retrain as v12, targeting 215/227 (95%) on the profile eval.

### Constraints
- FTMB format bumps to v4 (additive, backward-compatible with v1-v3 readers)
- Validation features are 239-dim (one pass rate per type, pre-computed)
- Numeric range validation added for latitude/longitude (string → float → range check)
- ISO 3166-1 alpha-2 enum added to country_code validation
- v11 model weights are NOT frozen — this is a full retrain
- Multi-branch architecture gains a 5th branch: validation(239) → Dense → Dense → merge

### Success Criteria
- Profile eval ≥ 215/227 (95% label accuracy)
- Actionability improves as a side-effect (mispredictions are the cause of 96.7%)
- All existing tests pass (374+)
- FTMB v4 reads backward-compatible v1-v3 files

### Decisions Surfaced
- **Validation as model feature, not Sharpen rule:** The validation pass rates are rich enough to be a model input feature, not a post-hoc override. The model learns cross-branch interactions (e.g., "validation says country_code but header says country" — let the model weigh it).
- **Full 239-dim, not category-level:** Keep maximum information. The model's linear layer handles dimensionality reduction.
- **Pre-compute into FTMB:** One-time cost during data generation. Training loop stays fast.
- **Add numeric range constraints:** Extends validation beyond string patterns to numeric types (latitude, longitude). Required to close the latitude↔decimal_number gap.
- **Country_code enum now:** Immediate ROI — makes the most common geographic misclassification (country↔country_code) directly solvable by the validation branch.

### Open Questions
- Exact FTMB v4 header layout — how many bytes for valid_dim and range features
- How to implement numeric range validation in the feature extraction pipeline (parse + check vs. separate pass)
- Whether prepare_multibranch_data.py computes validation features or delegates to `finetype extract-features`
- Training hyperparameters for v12 (hidden dims for validation branch, learning rate, etc.)

### Evidence Base
- 7 prior specs document the full progression from architecture to ceiling
- Validation infrastructure exists: jsonschema 0.42, CompiledValidator, validate_value(), SCHEMA_CACHE
- FTMB versioning pattern (v1→v2→v3) is clean and additive
- Schema discrimination confirmed for 7/8 top misclassifications (jwt pattern rejects user_agent; email_display requires angle brackets; etc.)
- Latitude↔decimal_number is the one pair requiring numeric range extension
