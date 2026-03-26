# AC-7: Sharpen Error Analysis

**Date:** 2026-03-26
**Baseline:** sherlock-v5-current, 155/190 (81.6% label)

## Error Pattern Analysis (35 misses)

### Pattern 1: hs_code false positives (6 misses, 17%)
Decimal numbers from earthquakes and weather data predicted as `hs_code`.
Values like `4.5`, `2.3`, `0.54` match the 2-level HS code format `XXXX.XX`.

**Root cause:** Generator produced 2-level HS codes (10% of output) which are
indistinguishable from plain decimals without header context.

**Fix applied:** AC-2 (C-01) — removed 2-level HS codes from generator + validation.
Retraining should eliminate these. No Sharpen rule needed.

### Pattern 2: version false positives (2 misses, 6%)
Iris dataset `sepal_length`/`sepal_width` predicted as `version`. Values like
`1.4`, `3.5` could match version patterns.

**Root cause:** Decimal values in narrow range (0-8) overlap with version major.minor.
**Resolution:** Model should learn from header context. No rule needed — per decision 0038.

### Pattern 3: Phone/SSN confusion (3 misses, 9%)
Phone numbers predicted as SSN in ecommerce_orders, people_directory, api_users_json.

**Root cause:** US phone numbers (10 digits) have structural similarity to SSN (9 digits).
Header disambiguation exists via Sharpen but confidence too high for SSN to override.
**Resolution:** Retraining with better phone/SSN diversity. Existing Sharpen rule
`phone_header_pattern` should help if model confidence is closer to threshold.

### Pattern 4: ean/upc confusion (2 misses, 6%)
UPC predicted as EAN (or vice versa). These are structurally identical (EAN-13 ⊃ UPC-A).

**Root cause:** Genuine ambiguity — UPC-A is a subset of EAN-13.
**Resolution:** Consider collapsing or adding accepted_labels mapping. Not a model defect.

### Pattern 5: Datetime format misses (5 misses, 14%)
Various datetime formats mis-predicted: eu_dot_date→iso_8601, year→compact_ym,
abbreviated_month_date→abbrev_month_no_comma.

**Root cause:** Datetime is the largest domain (84 types) with many overlapping formats.
**Resolution:** Eval expansion (AC-4) added 12 new datetime types. Retraining with
expanded eval coverage should improve. For `iso` vs `iso_date` (T-separator check),
a Sharpen rule is justified per the spec.

### Pattern 6: Miscellaneous single misses (17 misses, 49%)
Various one-off confusions: icao→unlocode, issn→ein, port→ean, etc.

**Root cause:** Mix of genuine ambiguity and insufficient training diversity.
**Resolution:** Retraining with expanded eval set addresses most of these.

## Summary

```
| Category              | Misses | % of total | Fix strategy          |
|-----------------------|--------|------------|-----------------------|
| hs_code FP (decimal)  | 6      | 17%        | ✅ Generator fix (AC-2)|
| version FP (decimal)  | 2      | 6%         | Retraining            |
| phone/SSN confusion   | 3      | 9%         | Retraining + diversity|
| ean/upc ambiguity     | 2      | 6%         | Accept mapping        |
| datetime format       | 5      | 14%        | Retraining + eval     |
| misc single           | 17     | 49%        | Retraining            |
```

## Sharpen Rule Recommendation

Per decision 0038 (prefer retraining over new rules):

- **No new rules recommended at this time.** The dominant error pattern (hs_code FP)
  is fixed at the generator level. Retraining should address most remaining errors.
- **Post-v6 candidate:** iso vs iso_date T-separator check — justified by spec,
  meaningful for analysts, structurally deterministic. Defer until post-v6 eval reveals
  if the model can learn this pattern.

## Expected Impact of Retraining

Conservative estimate: 6 hs_code FP eliminated + 3-5 miscellaneous improved via expanded
training data = **~164-170/190** target (from 155/190 baseline).
