# Interview: Retraining Spike — CharCNN on Distilled Data

**Date:** 2026-03-20
**Interviewer:** Nightingale

---

## Context

Tier 2 benchmark (PR #17) landed with 2,490 columns across 249 types. Baseline: 77.6% overall (93.6% agreement, 18.6% disagreement, 94.2% synthetic). The 18.6% disagreement accuracy is the headline gap — FineType misclassifies ~81% of what Claude catches. Decision 0038 established the retraining path over adding more heuristic rules.

Current model: char-cnn-v14-250, trained on ~372K synthetic samples, 10 epochs, pure Rust training (Decision 0009). Available distilled data: 85K adjudicated rows from Sherlock (58K qualifying with ≥5 values, 26K with <5 values).

The spike aims to answer: does training on real-world distilled data improve accuracy, and which data mix works best?

## Interview Q&A

### Q1: Training data scope
**Q:** The distilled data gives us 58K qualifying rows (≥5 values) plus 26K with <5 values that were excluded from the benchmark. For training, individual values are still useful. Should we use all 85K rows or only the 58K?
**A:** Investigate first. Compare label quality between the two sets before deciding — quick analysis of agreement rates and confidence distributions.

### Q2: Data shape
**Q:** The CharCNN classifies individual strings, but distilled data is labelled at the column level. To create training samples, we'd explode each column into individual value→label pairs. A column with 50 values creates 50 training samples, all with the same label. Does this approach seem right?
**A:** Value-level explosion. Matches how synthetic data works. Simple, high volume.

### Q3: Blend ratio
**Q:** For the blended mix, what's your intuition on the ratio? Distilled data is heavily skewed (categorical: 10.8K columns, entity_name: 10.3K) while synthetic generators produce balanced data.
**A:** Let the spike explore ratios. Particularly concerned about over-representing categorical, which depends heavily on column cardinality in the real world — a property the CharCNN can't see from individual values.

### Q4: Column-level types
**Q:** Should certain types be excluded from distilled training data because they're inherently column-level concepts? Categorical (cardinality), ordinal (ordering), and increment (sequential pattern) are diagnosed by column-level features, not character patterns.
**A:** Include but investigate. Measure whether adding distilled categorical/ordinal/increment data helps or hurts those specific types. Data-driven exclusion if the evidence warrants it.

### Q5: Success bar
**Q:** What improvement on disagreement accuracy would tell you the retraining path is worth pursuing?
**A:** Any improvement. If disagreement moves from 18.6% to even 25%+, the signal is positive. This is a spike — testing direction, not shipping a final model.

### Q6: Architecture scope
**Q:** Should the spike hold architecture constant and only vary training data, or also explore hyperparameters/architecture changes?
**A:** Data-only. Hold architecture and hyperparameters constant. Clean experiment isolating the effect of real-world data. Architecture changes are a separate spike if needed.

### Q7: Automation
**Q:** The spike needs 3–6 training runs (~45 min each on Metal), each with Tier 1 + Tier 2 scoring. Fully automated, semi-automated, or manual?
**A:** Fully automated. One script runs all experiments end-to-end, logs results to a comparison table.

---

## Summary

### Goal
Run a controlled experiment: train CharCNN on synthetic-only (baseline), distilled-only, and multiple blended mixes, measuring each against Tier 2 benchmark and Tier 1 profile eval. Identify which data mix improves real-world accuracy (especially disagreement rows) without regressing on synthetic or profile eval.

### Constraints
- Pure Rust training pipeline (Decision 0009) — Python only for data preparation
- Architecture and hyperparameters held constant (CharCNN, 10 epochs, same config as v14)
- Tier 1 profile eval (170/174, 97.7%) is the regression floor
- Distilled data quality investigation required before finalising training data scope
- Column-level types (categorical, ordinal, increment) included but monitored for negative transfer
- Spike is investigative — no commitment to ship a retrained model

### Success Criteria
- Comparison table: accuracy by mix across Tier 2 (overall, agreement, disagreement, synthetic, per-domain) and Tier 1
- Any improvement in disagreement accuracy (>18.6%) is a positive signal
- Tier 1 does not regress below 170/174 on the best-performing mix
- Clear signal on whether blended training improves common-type accuracy
- Data on whether column-level types (categorical, ordinal, increment) help or hurt when included
- Identification of which blend ratio (if any) outperforms synthetic-only

### Open Questions
- Exact quality comparison between 85K full rows vs 58K qualifying rows (pre-step investigation)
- Which blend ratios to explore (50/50, 70/30 distilled-heavy, 30/70 synthetic-heavy?)
- Whether to cap dominant types (categorical, entity_name) or let natural distribution stand
- How to handle the 128 types with zero distilled data in distilled-only runs (fall back to synthetic?)

### Sequencing
```
Pre-step: Investigate 85K vs 58K data quality ──────────► immediate
Step 1:   Prepare training data mixes ──────────────────► after investigation
Step 2:   Run automated experiment loop ────────────────► after data prep
Step 3:   Analyse results, write findings ──────────────► after all runs complete
Future:   Rule removal spike (Spec 3) ─────────────────► after this spike
```
