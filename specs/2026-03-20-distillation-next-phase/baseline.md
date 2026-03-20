# Tier 2 Benchmark — Baseline Results

**Date:** 2026-03-20
**Model:** char-cnn-v14-250 + Sense→Sharpen pipeline (v0.6.12)
**Benchmark:** eval/tier2_benchmark.csv (2,490 columns, 249 types, seed 42)
**Note:** identity.person.password excluded (no generator, no distilled data)

## Overall

| Metric | Score |
|--------|-------|
| **Overall accuracy** | **1,933/2,490 (77.6%)** |

## By Source

| Source | Correct | Total | Accuracy |
|--------|---------|-------|----------|
| Distilled | 320 | 777 | 41.2% |
| Synthetic | 1,613 | 1,713 | 94.2% |

## By Agreement Status

| Status | Correct | Total | Accuracy |
|--------|---------|-------|----------|
| Agreement (yes) | 219 | 234 | 93.6% |
| Disagreement (no) | 101 | 543 | 18.6% |
| Synthetic | 1,613 | 1,713 | 94.2% |

## By Domain

| Domain | Correct | Total | Accuracy |
|--------|---------|-------|----------|
| container | 98 | 120 | 81.7% |
| datetime | 699 | 840 | 83.2% |
| finance | 271 | 310 | 87.4% |
| geography | 180 | 250 | 72.0% |
| identity | 240 | 330 | 72.7% |
| representation | 199 | 360 | 55.3% |
| technology | 244 | 280 | 87.1% |

## Accuracy Distribution

| Accuracy band | Types |
|---------------|-------|
| 0% (complete failure) | 24 |
| 1–49% | 25 |
| 50–99% | 55 |
| 100% (perfect) | 145 |

## Key Findings

### The agreement/disagreement split is the headline number

- **93.6%** on agreement rows — FineType handles these well (by definition)
- **18.6%** on disagreement rows — FineType misclassifies ~81% of what Claude catches
- **94.2%** on synthetic — FineType classifies its own generated data accurately

This validates Decision 0039 (use all adjudicated rows): the agreement-only benchmark
would have shown ~94% accuracy, masking the real problem. The disagreement rows reveal
that FineType's effective accuracy on ambiguous real-world data is ~19%.

### Worst-performing types (0% accuracy, all distilled)

These are types FineType **never** classifies correctly in real-world data:

| Type | Source | Notes |
|------|--------|-------|
| geography.location.city | distilled | Misclassified as entity_name (headerless) |
| geography.address.street_name | distilled | Misclassified as entity_name |
| geography.coordinate.dms | distilled | Misclassified as text/numeric |
| identity.person.username | distilled | Misclassified as categorical/alphanumeric |
| identity.person.first_name | distilled | Misclassified as entity_name |
| identity.commerce.isbn | distilled | Misclassified as numeric_code |
| representation.text.plain_text | distilled | Misclassified as entity_name |
| representation.text.word | distilled | Misclassified as entity_name/categorical |
| representation.identifier.increment | distilled | Misclassified as integer_number |
| representation.identifier.alphanumeric_id | distilled | Misclassified as various |
| representation.numeric.integer_number | distilled | Misclassified as various numeric |
| representation.discrete.ordinal | distilled | Misclassified as categorical |
| finance.currency.amount | distilled | Misclassified as decimal_number |
| finance.currency.amount_comma | distilled | Misclassified as decimal_number_comma |
| datetime.time.hm_24h | distilled | Misclassified as hms_24h |
| datetime.epoch.unix_seconds | synthetic | Not detected at all |

### Representation domain is weakest (55.3%)

The representation domain covers categorical, ordinal, text subtypes, identifiers,
and numeric subtypes — all disambiguation-heavy types that depend on rules rather
than clear format patterns. This is the primary retraining target.

### What retraining needs to improve

The 49 types with <50% accuracy are the retraining spike's target list. If a
retrained model can move the disagreement accuracy from 19.2% toward 50%+, that
alone would represent a major improvement. The agreement and synthetic rows should
remain stable (regression floor).
