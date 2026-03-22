# Interview: Multi-Branch Production Path

**Date:** 2026-03-22
**Interviewer:** Nightingale

---

## Context

Following the overnight Sherlock ablation (2026-03-21), multi-branch was identified as the right successor to CharCNN for FineType's primary inference pipeline. Multi-branch offers a fundamentally faster inference path (deterministic features + single MLP vs 100 CNN forward passes + vote aggregation + disambiguation rules), but the ablation showed it was data-starved (28.5k column-level samples) and the eval harness can't load multi-branch models for Tier 1 comparison.

This interview scopes the work to make multi-branch production-ready, including implications for single-value classification.

**Prior art:**
- `specs/2026-03-21-sherlock-architecture/spec.yaml` — Original Sherlock architecture spec (AC-1 through AC-10)
- `specs/2026-03-21-sherlock-architecture/ablation-results.md` — Overnight ablation results
- `specs/2026-03-20-retraining-spike/findings.md` — Retraining spike proving blend-30-70 optimal

## Interview Q&A

### Q1: Single-value classification
**Q:** When multi-branch becomes the default production model, what should happen to single-value classification (`finetype infer -i`)? Today CharCNN handles this directly. Multi-branch needs a column of values to compute its 1,499-dim features. What's your expectation?

**A:** The degrade gracefully approach makes sense. Reframe `infer` to be array-oriented that also accepts a string value. A single-member array would be less accurate (lower signal) and we can document it. Thinking would only change if accuracy dropped significantly — enough to impact the product.

**Analysis (Nightingale):** For n=1, ~80% of the 1,499-dim input collapses to zero/degenerate values (variance, skew, kurtosis, entropy, uniqueness all degenerate). Format-detectable types (email, date, UUID) would likely survive on char frequency alone. Semantic types (entity_name vs plain_text vs categorical) would likely break. Mitigation: keep CharCNN as lightweight fallback for n=1 inputs (250KB model).

### Q2: Success metric
**Q:** What accuracy target should multi-branch hit on the Tier 1 profile eval before it replaces CharCNN as default?

**A:** Close enough (≥90%) + speed win. If multi-branch is within 3-4pp of CharCNN but significantly faster, the speed advantage justifies the switch. Accuracy can improve iteratively.

### Q3: Data scaling strategy
**Q:** To hit ≥90% Tier 1, we likely need 10-50x more column-level training data. Which scaling strategy?

**A:** Both in a blend — same blend strategy that worked for CharCNN: 30% distilled (bootstrapped) + 70% synthetic columns.

### Q4: Sequencing
**Q:** Should we tackle the eval harness gap FIRST (before retraining), so we can measure progress on every subsequent training run?

**A:** Eval first, then retrain. Wire multi-branch into `finetype profile` first so every training experiment gets a real Tier 1 score.

### Q5: Integration architecture
**Q:** How should multi-branch integrate with `finetype profile`?

**A:** Multi-branch becomes the Sense model. This was discussed in the Sherlock architecture spec: "Redefine the pipeline so Sense = this model, Sharpen = deterministic corrections." It's a product concept, not a pipeline imperative.

### Q6: Accuracy standard
**Q:** The original spec targets 85% Tier 2 with 80% rollback. You said ≥90% Tier 1. Should we update?

**A:** "I always zoom out to my ultimate goal which is to 'spark joy' for analysts — this means ≥95% accuracy. This is a pillar of the finetype project and meridian broadly."

**Agreed recommendation:** ≥95% Tier 1 label accuracy as the ship gate. Tier 2 used for per-type regression analysis but not a ship/no-ship decision.

**Context:** Current production CharCNN scores 97.7% Tier 1. The v16 baseline (blend-30-70, 10 epochs) scored 93.7%. The ≥95% target means multi-branch must close to within 2.7pp of production.

### Q7: Label remapping
**Q:** Should we remap the 29 non-canonical distilled labels (~131 columns) to canonical taxonomy paths?

**A:** Yes — leverage the data we already have. No new distillation needed (lots done this week). Further distillation possible but needs to wait until after next Friday.

### Q8: Timeline
**Q:** What's the realistic timeline expectation?

**A:** Milestone-driven, not time-boxed. Ship each piece as it's ready. Don't set a deadline — set quality gates.

### Q9: Head type focus
**Q:** Flat won the ablation (60.3% vs 56.5%, 3x faster). Focus exclusively on flat, or keep hierarchical in play?

**A:** Revisit after data scaling. The hierarchical head lost at 28.5k samples; at 300k+ it might close the gap since it has structural advantage for the taxonomy tree. Don't decide until we have more data.

---

## Summary

### Goal
Make multi-branch the production successor to CharCNN as FineType's primary column-type classifier. Multi-branch replaces the Sense stage; existing Sharpen rules (F1-F6, locale detection, leading-zero, etc.) preserved as deterministic post-processing. Single-value inference degrades gracefully (CharCNN fallback for n=1 if needed).

### Constraints
- Eval harness integration FIRST — must be able to run Tier 1 profile eval on multi-branch models before any retraining
- Blend-30-70 column-level data (30% distilled + 70% synthetic columns), scaled from 33k to 300k+
- Remap 29 non-canonical distilled labels to recover ~131 columns of real-world signal
- No new distillation until after next Friday (2026-03-28)
- M1 Pro training hardware (Metal acceleration)
- Milestone-driven, not time-boxed — each piece ships when it passes its quality gate
- Flat head is primary; hierarchical head revisited after data scaling

### Success Criteria
- **Ship gate:** ≥95% Tier 1 label accuracy on 30 real-world CSVs (production CharCNN = 97.7%)
- **Speed gate:** Demonstrably faster inference than CharCNN Sense→Sharpen pipeline
- **Regression guard:** No individual type regresses by more than 3 columns on Tier 2
- **Graceful degradation:** Single-value inference works (lower accuracy documented and acceptable)

### Milestones (ordered)
1. **Eval harness:** Wire multi-branch model loading into `finetype profile` so Tier 1 eval runs
2. **Baseline measurement:** Run existing 28.5k-trained flat model through Tier 1 — establish true multi-branch accuracy (not just val accuracy)
3. **Label remapping:** Map 29 non-canonical labels to canonical taxonomy, recover ~131 columns
4. **Synthetic column generation:** Build pipeline to generate column-level feature vectors from FineType's generators at scale
5. **Data scaling:** Produce 300k+ column-level blend-30-70 training data
6. **Retrain flat:** Train with scaled data, more epochs, measure Tier 1
7. **Revisit hierarchical:** If flat doesn't hit 95%, try hierarchical with scaled data
8. **Production integration:** Replace Sense stage, wire Sharpen, ship as default

### Open Questions
- Exact synthetic column generation approach (how to generate realistic column-level features from per-type generators)
- Whether CharCNN fallback for n=1 is needed or if multi-branch degrades acceptably
- How many epochs needed with scaled data (28.5k models were still improving at epoch 10)
- Whether sibling-context (AC-6b/d, deferred) adds value once data scaling is solved
