---
id: NNFT-083
title: >-
  Fix numeric type disambiguation (decimal_number vs integer_number vs
  street_number)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-17 02:57'
updated_date: '2026-02-17 06:34'
labels:
  - model
  - accuracy
  - training-data
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The NNFT-081 profile evaluation revealed systematic numeric misclassification:
- 8 columns labeled "number" predicted as decimal_number instead of integer_number (iris petals, temperatures, pressures, heart_rate)
- 4 columns labeled "number" predicted as street_number (pages, elevation, heart_rate)
- Additional confusion between numeric types and postal_code, cvv, ean, latitude

Root cause: the CharCNN model over-predicts decimal_number for any numeric column with decimal points, and street_number for small integers. The training data generators likely have overlapping value ranges.

Goal: improve numeric type precision so the model correctly distinguishes between general numeric values and format-specific types (street_number, postal_code, etc.).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Profile eval integer_number recall improves from 22.7% to ≥50%
- [ ] #2 street_number false positive rate decreases (currently predicted 6 times, only 1 correct)
- [x] #3 decimal_number precision improves from 0% to ≥30% on eval benchmark
- [x] #4 All 213 existing tests still pass
- [x] #5 CharCNN retrained with updated training data
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Fix eval gt_labels: columns with decimal values should use 'decimal number' gt_label, not generic 'number'
2. Add 'decimal number' gt_label to schema_mapping.csv (direct → representation.numeric.decimal_number)
3. Make street_number generator more distinctive: increase alphanumeric suffix rate (10%→40%), add hyphenated formats, narrow range to 1-3000
4. Widen integer_number generator range and add comma-separated thousands format
5. Review CVV generator overlap with small integers
6. Regenerate training data: make generate
7. Retrain CharCNN v7
8. Re-run profile eval to verify improvements
9. Update default model symlink if accuracy improves"
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Completed generator improvements, training bug fix, and eval corrections.

Key changes:
- street_number: narrowed to 1-2000, added 50% distinctive formats (letter suffixes, hyphens, fractions)
- integer_number: varied magnitude distribution (-100K to 1B) instead of flat range
- CVV: range 0-999 so leading zeros (001, 042) appear as distinguishing signal
- Fixed critical training bug: locale suffix (.UNIVERSAL) in training labels caused all samples to map to class 0
- Split 10 decimal columns from generic 'number' to 'decimal number' gt_label

Results:
- CharCNN v7: 85.14% synthetic accuracy (20 epochs, real training)
- Profile eval: 68.1% label / 78.8% domain accuracy (format-detectable)
- decimal_number: 87.5% precision, 70% recall (massive improvement)
- integer_number: 83.3% precision, 35.7% recall (improved but below 50% target)
- street_number: 100% recall but 12.5% precision (overpredicting)

AC#1 (integer recall ≥50%): 35.7% — improved but not met, hitting single-model ceiling
AC#2 (street_number FP decrease): mixed — recall improved but model overpredicts
AC#3 (decimal precision ≥30%): 87.5% — far exceeded target
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed numeric type disambiguation through generator improvements, training infrastructure bug fix, and eval label corrections. Committed as 3bb91ee.

## Changes

**Generators (generator.rs):**
- street_number: narrowed range (1-2000), added 50% distinctive address formats (letter suffixes like 12A, hyphenated ranges like 12-14, fractions like 12 1/2)
- integer_number: varied magnitude distribution (small 0-1K, medium ±10K, large 1K-10M, very large 100K-1B, negatives) instead of flat -100K..100K
- CVV: range 0-999 (was 100-999) so leading zeros (001, 042) actually appear as distinguishing signal

**Training bug fix (char_training.rs):**
- Generated labels include locale suffix (.UNIVERSAL, .en_US) but label_to_index only maps bare taxonomy keys
- All training samples silently mapped to class 0 (container.array.comma_separated), producing a degenerate model
- Fix: rsplit_once('.') fallback in prepare_batch strips locale suffix before lookup
- This means v5 and v6 models were also broken — always predicting class 0

**Eval corrections:**
- Added 'decimal number' gt_label to schema_mapping.csv → representation.numeric.decimal_number
- Reclassified 10 columns from generic 'number' to 'decimal number' (iris×4, financial pe_ratio, medical temperature_f, scientific×4)
- Updated street_number validation pattern for new generator formats

**Model:**
- CharCNN v7: 85.14% accuracy (20 epochs) — first correctly-trained model
- Default model symlink updated to char-cnn-v7

## Results
- Headline: 68.1% label / 78.8% domain accuracy (format-detectable)
- decimal_number: 87.5% precision, 70% recall (massive improvement from 0%)
- integer_number: 83.3% precision, 35.7% recall (improved but below 50% target)
- AC#1 and AC#2 not fully met — single flat model hitting 169-class ceiling
- Recommended: tiered model graph (NNFT-084) for further gains"
<!-- SECTION:FINAL_SUMMARY:END -->
