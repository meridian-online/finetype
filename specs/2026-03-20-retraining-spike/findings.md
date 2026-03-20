# Retraining Spike — Findings

**Date:** 2026-03-20
**Spec:** specs/2026-03-20-retraining-spike/spec.yaml
**Branch:** feat/retraining-spike

---

## Executive Summary

Blending distilled real-world data with synthetic training data **improves Tier 2 accuracy across every metric**. The best mix (blend-30-70: 30% distilled, 70% synthetic) achieved **80.6% overall** (up from 75.0% synthetic-only baseline), with disagreement accuracy nearly doubling from 16.8% to 33.0%.

51 types improved, only 17 regressed. Every domain improved except geography (-2, within noise). The signal is clear: real-world data helps, and the optimal ratio favours synthetic.

**Recommendation:** Retrain the production model with blend-30-70 data mix after completing distillation (currently 62% done). More distilled data will strengthen the signal further.

---

## Experiment Configuration

All experiments held constant: CharCNN flat architecture, embed=32, filters=64, hidden=128, 10 epochs, batch 32, seed 42. Only the training data varied.

```
| Experiment             | Description                                      | Samples/type |
|------------------------|--------------------------------------------------|-------------|
| synthetic              | Pure synthetic baseline (same as char-cnn-v14)   | 1,500       |
| distilled-backfill     | Max distilled per type, synthetic fills remainder | 1,500       |
| blend-50-50            | 50% distilled + 50% synthetic per type           | 1,500       |
| blend-70-30            | 70% distilled + 30% synthetic per type           | 1,500       |
| blend-30-70            | 30% distilled + 70% synthetic per type           | 1,500       |
| blend-70-30-no-coltype | blend-70-30 excluding categorical/ordinal/incr    | 1,500       |
```

Data source: 58,424 qualifying rows from Sherlock distillation v3 (≥5 values per column, valid taxonomy labels). 122 types with distilled coverage, 248 types total with synthetic backfill.

---

## Results — Tier 2 Benchmark (2,490 columns, 249 types)

```
| Mix                    | Overall          | Agreement        | Disagreement     | Synthetic        | Distilled        |
|------------------------|------------------|------------------|------------------|------------------|------------------|
| synthetic (baseline)   | 1868/2490 (75.0%)| 185/234 (79.1%) | 91/543 (16.8%)  | 1592/1713 (92.9%)| 276/777 (35.5%) |
| distilled-backfill     | 1700/2490 (68.3%)| 194/234 (82.9%) | 176/543 (32.4%) | 1330/1713 (77.6%)| 370/777 (47.6%) |
| blend-50-50            | 1998/2490 (80.2%)| 195/234 (83.3%) | 177/543 (32.6%) | 1626/1713 (94.9%)| 372/777 (47.9%) |
| blend-70-30            | 1989/2490 (79.9%)| 199/234 (85.0%) | 186/543 (34.3%) | 1604/1713 (93.6%)| 385/777 (49.5%) |
| **blend-30-70**        |**2008/2490 (80.6%)**|**199/234 (85.0%)**| 179/543 (33.0%)| **1630/1713 (95.2%)**| 378/777 (48.6%) |
| blend-70-30-no-coltype | 1998/2490 (80.2%)| 198/234 (84.6%) | 183/543 (33.7%) | 1617/1713 (94.4%)| 381/777 (49.0%) |
```

### Key Observations

1. **All blends beat pure synthetic.** Even blend-70-30 (most distilled) improves overall from 75.0% to 79.9%.

2. **Blend-30-70 is the overall winner** at 80.6% overall (+5.6pp), but blend-70-30 wins on disagreement (34.3%) and distilled accuracy (49.5%).

3. **Disagreement accuracy nearly doubled** across all blends (16.8% → 32-34%), confirming the hypothesis that real-world data teaches the model to handle ambiguous cases.

4. **Synthetic accuracy is preserved** in blends — blend-30-70 actually *improves* synthetic from 92.9% to 95.2%. No catastrophic forgetting.

5. **Distilled-only (with backfill) is the worst mix** at 68.3% overall. Too much noisy real-world data without sufficient synthetic variety hurts performance — the 15% synthetic drop (92.9% → 77.6%) is severe.

6. **Column-level types (categorical/ordinal/increment) have minimal impact.** Removing them (blend-70-30-no-coltype) barely changes results: 79.9% → 80.2% overall, 34.3% → 33.7% disagreement. Not the negative transfer concern we feared.

---

## Results — By Domain

Best mix (blend-30-70) vs synthetic-only baseline:

```
| Domain          | Baseline           | Blend-30-70        | Delta |
|-----------------|--------------------|--------------------|-------|
| container       |   99/120 (82.5%)   |  110/120 (91.7%)   |  +11  |
| datetime        |  685/840 (81.5%)   |  730/840 (86.9%)   |  +45  |
| finance         |  258/310 (83.2%)   |  276/310 (89.0%)   |  +18  |
| geography       |  177/250 (70.8%)   |  175/250 (70.0%)   |   -2  |
| identity        |  224/330 (67.9%)   |  239/330 (72.4%)   |  +15  |
| representation  |  189/360 (52.5%)   |  238/360 (66.1%)   |  +49  |
| technology      |  236/280 (84.3%)   |  240/280 (85.7%)   |   +4  |
```

Representation domain saw the largest gain (+49, +13.6pp). Geography is flat — Sherlock's geographic data is primarily headerless, making it genuinely hard.

---

## Results — Per-Type Analysis

**51 types improved, 181 unchanged, 17 regressed** (blend-30-70 vs synthetic baseline).

### Top Improvements (≥5 columns gained)

```
| Type                                   | Baseline  | Blend-30-70 | Delta |
|----------------------------------------|-----------|-------------|-------|
| representation.identifier.increment    |  0/10     |  10/10      |  +10  |
| identity.medical.npi                   |  0/10     |  10/10      |  +10  |
| datetime.epoch.unix_microseconds       |  0/10     |  10/10      |  +10  |
| finance.currency.amount_nodecimal      |  1/10     |  10/10      |   +9  |
| technology.internet.hostname           |  2/10     |  10/10      |   +8  |
| datetime.date.ymd_dot                  |  1/10     |   9/10      |   +8  |
| datetime.date.compact_mdy             |  2/10     |  10/10      |   +8  |
| representation.file.extension          |  2/10     |   9/10      |   +7  |
| representation.boolean.terms           |  1/10     |   8/10      |   +7  |
| datetime.timestamp.epoch_nanoseconds   |  0/10     |   6/10      |   +6  |
| finance.currency.amount                |  0/10     |   5/10      |   +5  |
| datetime.time.hm_24h                   |  0/10     |   5/10      |   +5  |
| datetime.date.short_dmy               |  3/10     |   8/10      |   +5  |
```

### Notable Regressions (≥3 columns lost)

```
| Type                                   | Baseline  | Blend-30-70 | Delta |
|----------------------------------------|-----------|-------------|-------|
| datetime.epoch.unix_seconds            | 10/10     |   0/10      |  -10  |
| geography.location.region              |  6/10     |   2/10      |   -4  |
| technology.code.imei                   | 10/10     |   6/10      |   -4  |
| representation.identifier.numeric_code |  5/10     |   2/10      |   -3  |
```

The unix_seconds regression (-10) is notable but likely fixable — it suggests the distilled data for epoch types has conflicting labels between seconds/milliseconds/microseconds granularity.

---

## Tier 1 Profile Eval

**Not captured.** A bug in the DuckDB output parser (Unicode box-drawing characters garbling shell variable expansion) caused Tier 1 scores to be garbage in comparison.csv. The spike models were ephemeral (not persisted after the experiment loop).

**Mitigation:** Tier 1 regression should be checked before promoting any blend model to production. This requires retraining the winning mix and running profile eval separately.

---

## Conclusions

### The hypothesis is confirmed

Real-world distilled data substantially improves Tier 2 accuracy. The improvement is broad (6 of 7 domains, 51 types improved), and blending preserves synthetic performance.

### Optimal ratio: 30% distilled, 70% synthetic

This maximises overall accuracy (80.6%) and synthetic performance (95.2%) while still capturing most of the disagreement improvement. Higher distilled ratios slightly improve disagreement accuracy but at the cost of overall and synthetic performance.

### Column-level types are not a problem

Excluding categorical/ordinal/increment from training had negligible effect. The feared negative transfer did not materialise.

### Distilled-only is a bad idea

Pure distilled (with synthetic backfill) drops overall accuracy to 68.3%. The model needs synthetic data for variety and type coverage.

---

## Recommended Next Steps

1. **Complete distillation pipeline** — Currently 62% done (852/1374 batches). More distilled data across more types will strengthen the signal.

2. **Fix Tier 1 regression check** — Retrain blend-30-70 and run full profile eval to confirm no Tier 1 regression before promoting.

3. **Investigate regressions** — The unix_seconds (-10) and region (-4) regressions should be root-caused. Likely conflicting labels in distilled data for epoch granularity types.

4. **Ship production model** — After Tier 1 verification, promote blend-30-70 trained model as char-cnn-v15 (or v16, accounting for the hierarchical v15).

5. **Fix run_spike.sh Tier 1 parser** — Replace DuckDB piped output parsing with JSON output mode or direct file parsing to avoid Unicode garbling.
