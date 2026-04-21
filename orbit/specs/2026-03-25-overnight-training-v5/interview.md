# Interview: Overnight Training Run v5 — Generator Audit, Augmentation, Training Expansion

**Date:** 2026-03-25
**Interviewer:** Nightingale

---

## Context

Before cutting v0.6.13, Hugh wants to run a significant overnight training run to push multi-branch model accuracy up. The current raw model scores 121/190 (63.7%), with Sharpen rules rescuing 34 more to 155/190 (81.6%). The goal is to reduce dependence on rules by improving the model itself.

Pre-interview research revealed:
- All 250/250 types have working generators (100% coverage)
- No value-level augmentation exists in the training pipeline
- 71 types (29%) have zero distilled data; 103 (41%) have <10 rows
- Distilled data is LLM-labelled (Claude blind-first adjudication) with no human audit
- The #1 eval failure is `hs_code` false positives (6 decimal columns misclassified) due to generator format collision
- The model was trained on 50/50 distilled/synthetic blend, ~194k records, 20 epochs

## Interview Q&A

### Q1: Primary Goal
**Q:** The raw model scores 121/190 — Sharpen rules rescue 34 more to reach 155. What's your primary goal: push raw model accuracy up (reducing rule dependence), or push the post-Sharpen total higher?
**A:** Raw model accuracy. Target the model itself so rules become a safety net, not load-bearing.

### Q2: Data Gap Analysis
**Q:** Where do you think the biggest data gap is — hard negatives, distribution imbalance, or generator realism?
**A:** Research this to determine the answer. Also check if distilled labels are accurate.

**Research findings:**
- **Distribution imbalance (worst):** 71 types have zero distilled data, 103 have <10 rows. The model trains on pure synthetic for 29% of types.
- **Generator collisions (structural):** `hs_code` produces `XXXX.XX` indistinguishable from `decimal_number`. `latitude` fixed 4-decimal. `version` produces values like `4.9`. These are the direct cause of the top eval failures.
- **Hard negatives (absent):** No adversarial pairing in training. The model never learns decision boundaries between confusable types.
- **Distillation quality:** LLM-labelled, no human audit. 72% disagreement rate (expected — FineType was wrong more often). Spot checks look defensible but no formal accuracy assessment exists.
- Password generator confirmed: exists at `identity.person.password`, generates 8-20 char alphanumeric + special chars.

### Q3: Scope
**Q:** Which intervention layers for the overnight run?
**A:** All three — fix generators, add augmentation, mine hard negatives.

### Q4: Compute Budget
**Q:** How to spend the ~8 hour overnight budget?
**A:** More data AND more epochs. Increase samples_per_type to 3000+ and train 40 epochs.

### Q5: Blend Ratio
**Q:** Change the 50/50 distilled/synthetic blend ratio?
**A:** Keep 50/50 with adaptive fill — when distilled < target, fill with augmented synthetic. Preserves real-world signal where available.

### Q6: Model Architecture
**Q:** Scale up the model size? Current arch is modest (trunk 500→500→250).
**A:** Initial question about whether inference time would be impacted.

**Research finding:** Model size is essentially free for inference. Multi-branch does one forward pass per column (not per value). Feature extraction is the bottleneck, not the classifier head. A 50% wider model adds sub-millisecond overhead.

### Q6b: Scale-up Decision
**Q:** Given model size is free, include a scale-up experiment?
**A:** Train both current arch and ~50% wider, compare. Adds ~3 hours but answers the capacity question definitively.

### Q7: Hard-Negative Strategy
**Q:** How aggressive with hard-negative injection?
**A:** Oversampled confusion types — 2-3x samples_per_type for top-10 confused types. Simpler than adversarial table assembly.

### Q8: Augmentation Rate
**Q:** What percentage of training samples get noise?
**A:** 30-40% of samples augmented. Aggressive — real-world data is messy.

### Q9: Label Audit
**Q:** Validate distilled labels before training?
**A:** Quick validation pass — run generator-based format check, flag implausible labels for exclusion.

### Q10: Success Criterion
**Q:** What's the target?
**A:** Raw 140+/190, post-Sharpen 170+/190.

---

## Summary

### Goal
Push raw multi-branch model accuracy from 121/190 to 140+/190 (and post-Sharpen from 155 to 170+/190) through improved training data quality, augmentation, and increased model capacity.

### Constraints
- Single overnight run (~8 hours on M1 Pro with Metal)
- Must not regress post-Sharpen accuracy below 155/190
- Preserve 50/50 distilled/synthetic blend with adaptive fill
- Two model variants (current arch + scaled-up) trained on same data for comparison

### Success Criteria
- Raw model accuracy ≥ 140/190 on profile eval
- Post-Sharpen accuracy ≥ 170/190 on profile eval
- Best-of-two architectures ships as new default

### Interventions (all in scope)
1. **Generator fixes:** Fix collision generators (hs_code, latitude, version, ssn) to produce structurally distinct output
2. **Value-level augmentation:** 30-40% of samples get controlled noise (whitespace, encoding artifacts, format mixing, typos)
3. **Hard-negative oversampling:** 2-3x samples for top-10 confused types from eval failures
4. **Quick label audit:** Validate distilled labels against generator format patterns, exclude implausible labels
5. **Scale-up experiment:** Train both current arch and ~50% wider, compare

### Training Parameters
- samples_per_type: 3000-5000 (up from 1200)
- epochs: 40 (up from 20)
- blend: 50/50 with adaptive fill
- augmentation rate: 30-40%
- confusion type oversampling: 2-3x for top-10 confused pairs
- two architectures: current (trunk 500→500) + scaled (trunk 750→750)

### Open Questions
- Exact augmentation types to implement (whitespace, encoding, typos, format mixing — which subset?)
- Specific generator changes needed for each collision type
- Whether sibling-context should be retrained or frozen from v4
