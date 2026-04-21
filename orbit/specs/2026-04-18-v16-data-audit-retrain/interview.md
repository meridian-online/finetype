# Discovery: v16 Data Audit and Retrain

**Date:** 2026-04-18
**Interviewer:** Nightingale
**Card:** none (sprint-level initiative)
**Mode:** discovery

---

## Context

At 218/227 (96.0%) profile eval accuracy, the 9 remaining misclassifications are all model errors that rules cannot fix. An earlier actionability audit revealed that the iso_8601 catch-all was masking prediction errors — columns like us_date, eu_date, and clf_timestamp were being predicted as iso_8601, and the format_string failures were misattributed to empty format_strings rather than misclassification. The training data (sherlock_distilled, 102k rows, 176 labels) was last audited in v12 but the iso_8601 masking effect means some mislabeled examples likely survived. Additionally, 64 of 240 taxonomy types have zero training examples.

## Q&A

### Q1: Audit scope
**Q:** The training data (sherlock_distilled, 102k rows) was audited in v12 with distilled data decontamination and a 600/type cap. For v16, what's your appetite for the audit scope — just fix the known confusion pairs (phone/ssn, user_agent/jwt, etc.) in the training data, or do a broader sweep looking for any mislabeled examples across all 176 labels?
**A:** Comprehensive — systematic audit across all 176 labels in training data. Use the model itself to flag disagreements between training labels and current predictions.

### Q2: Audit method
**Q:** For the comprehensive audit, the most efficient approach is to use the current v14 model as a critic — run inference on every training example, flag where the model's prediction disagrees with the training label, then human-review the disagreements. Sound right, or do you have a different audit method in mind?
**A:** Model-as-critic — run v14 inference on all 102k training rows. Flag disagreements. Fixes flow both ways.

### Q3: Eval audit
**Q:** The eval dataset (manifest.csv, 338 ground truth labels across 35 datasets) also needs auditing. Should we audit the eval ground truth with the same model-as-critic method, or manually review all 338 eval labels?
**A:** Manual review all 338 — the set is small enough for full manual review, and eval ground truth errors are higher-stakes.

### Q4: Adjudication method
**Q:** For the training data audit — when the model disagrees with the training label, who adjudicates? With 102k rows there could be thousands of disagreements.
**A:** Heuristic + sample — use validation patterns to auto-adjudicate where possible (if values match type X's regex but not type Y's, X wins). Manual-review a sample of the remainder.

### Q5: Training config
**Q:** On the retrain itself — v14 was trained for 50 epochs (~127 min on Metal, best val_acc 91.2%). Any changes to the training setup?
**A:** Clean data + longer training — same 5-branch architecture but train for 75–100 epochs (~3–4 hours on Metal).

### Q6: Coverage gaps
**Q:** The training data has 176 labels but the taxonomy has 240 types. That means 64 types have zero training examples. Should the audit include generating synthetic training data for uncovered types?
**A:** Clean + fill all gaps — generate synthetic examples for all 64 uncovered types to maximise coverage.

### Q7: Synthetic volume
**Q:** How many synthetic examples per uncovered type?
**A:** 600 per type (match cap) — match the existing distilled data cap. 64 x 600 = ~38k new rows.

### Q8: Success bar
**Q:** What's the success bar for v16? Currently at 218/227 (96.0%).
**A:** 221+ (fix 3+ errors) — 97.4%+. Fix the clearer confusions where better training data should help directly.

---

## Summary

### Goal
Comprehensive audit of training and eval data quality, fill coverage gaps for 64 uncovered types, and retrain multi-branch model (v16) on clean data with longer training (75–100 epochs).

### Constraints
- Same 5-branch multi-branch architecture (no architecture changes)
- 600/type synthetic data cap (matching v12 convention)
- Training on Metal (~3–4 hours estimated)
- Model-as-critic for training data audit; full manual review for eval data

### Success Criteria
- Training data: all 176 existing labels validated, mislabeled examples corrected
- Eval data: all 338 ground truth labels manually verified
- Coverage: all 240 taxonomy types represented in training data
- Profile eval: 221+/227 (97.4%+, fix 3+ of 9 remaining errors)
- No regressions on currently correct columns

### Decisions Surfaced
- **Comprehensive over targeted audit**: chose comprehensive sweep over targeted fix for known confusions — the iso_8601 masking showed that unknown data quality issues can hide in plain sight
- **Heuristic + sample adjudication**: validation regex auto-adjudicates where possible, manual review for ambiguous cases — balances quality with practical effort
- **Fill all coverage gaps**: generate 600 synthetic examples for each of 64 uncovered types — maximise the model's exposure to the full taxonomy
- **Longer training**: 75–100 epochs vs 50 — more epochs to learn from broader/cleaner data

### Open Questions
- How many training data disagreements will the model-as-critic find? (determines audit effort)
- Will synthetic data for 64 new types cause any regression on existing types?
