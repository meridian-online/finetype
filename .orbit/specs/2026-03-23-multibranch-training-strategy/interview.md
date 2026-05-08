# Interview: Multi-Branch Training Strategy to Close the Accuracy Gap

**Date:** 2026-03-23
**Interviewer:** Nightingale

---

## Context

The multi-branch (Sherlock-style) model is the target architecture for FineType going forward —
it's proven faster than CharCNN. However, sherlock-v2-flat scores 128/190 (67.4%) on the profile
eval vs CharCNN's 188/190 (98.9%). The goal is to close this gap through better training, reducing
reliance on hand-tuned disambiguation rules.

Prior findings in `.orbit/specs/2026-03-23-multibranch-eval-diagnosis/findings.md` identified three factors:
- Factor A: Missing post-processing (50% of top errors fixable by header hints alone)
- Factor B: Distribution shift between training data and eval set
- Factor C: No header signal in the model's feature vector

## Interview Q&A

### Q1: Root cause of the gap
**Q:** The multi-branch model takes 3 feature branches (char 960d, embed 512d, stats 27d) but
does NOT receive the column header as input. Headers like 'npi', 'latitude', 'email' are the
strongest signal. What's the biggest root cause: missing header signal or training data mismatch?
**A:** "I don't know what the cause is. Please search the spec folder for findings and why we
didn't include the header. I believe we're also expanding the eval dataset."

*Findings doc (Section 1C) confirms: 5/10 top misclassifications fixable by header hints alone.
Model is architecturally blind to header signal.*

### Q2: Scope — architecture change or data only?
**Q:** The findings identified Phase 2 (add header as 4th branch) and Phase 3 (better training data).
Should we treat adding the header branch as part of this training work?
**A:** Yes — header branch + better data.

### Q3: Data strategy
**Q:** Current blend is 70% synthetic + 30% distilled. Should we include eval dataset columns
as training signal?
**A:** 50/50 distilled/synthetic + eval holdout. (Later revised — see Q6.)

### Q4: Accuracy target
**Q:** What accuracy target makes multi-branch shippable as default?
**A:** Prove the architecture: ≥150/190 (79%+). This is about proving the header branch adds
signal and the architecture is viable. Final production accuracy comes from iteration.

### Q5: Execution approach
**Q:** Implement code changes + run training end-to-end, or ship code and let you run overnight?
**A:** Implement + run end-to-end.

### Q6: Eval holdout strategy
**Q:** How to handle eval data contamination?
**A:** "Now I think we should just leave eval out." — No eval data in training. Keep eval set
as a clean, uncontaminated benchmark. Use 50/50 distilled/synthetic mix only.

---

## Summary

### Goal
Add a 4th header embedding branch to the multi-branch architecture and retrain with 50/50
distilled/synthetic data mix. Target: ≥150/190 (79%+) on profile eval, proving the architecture
is viable and header signal improves accuracy.

### Constraints
- No eval data in training (clean benchmark)
- 50/50 distilled/synthetic data mix (up from 30/70)
- M1 Pro training hardware (Metal acceleration)
- End-to-end execution: code changes + training run + eval in one session

### Success Criteria
- ≥150/190 (79%+) label accuracy on 190-column profile eval
- Improvement over sherlock-v2-flat baseline (128/190, 67.4%)
- Header branch demonstrably contributes (ablation: with vs without header)

### Open Questions
- None — requirements are clear
