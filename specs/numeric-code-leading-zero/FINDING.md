# Finding: numeric_code vs integer_number — Leading Zero Signal

**Date:** 2026-03-08
**Related:** NNFT-253/254 (feature-retrain spike), NNFT-250 (feature extractor)

## Observation

`duration_minutes` in `sports_events.csv` classified as `representation.identifier.numeric_code` (VARCHAR) instead of `representation.numeric.integer_number` (BIGINT), despite a hardcoded header hint for `duration_minutes` → `integer_number`.

Values: `60, 90, 120, 150, 180` — all pure digits, no leading zeros.

## Why the Header Hint Didn't Override

CharCNN predicts `numeric_code` at **100% confidence**. The header hint override logic requires `confidence < threshold` to fire:

- Cross-domain threshold: 0.85
- Same-domain threshold: 0.5

Both `numeric_code` and `integer_number` are in the `representation` domain → same-domain threshold applies → but 1.0 >> 0.5.

No override mechanism in the current pipeline can correct a 100%-confidence same-domain prediction.

## Root Insight

The `numeric_code` type exists specifically to **preserve leading zeros** (e.g., "036", "00123", NAICS codes, FIPS codes). If no values in a column have leading zeros, then `numeric_code` adds no value — the column is safely castable to an integer type.

The CharCNN cannot make this distinction because it classifies individual values without column-level context about leading-zero prevalence. A pure-digit string like "120" is indistinguishable from a 3-digit code at the value level.

## Potential Fix

A post-vote rule (similar to F1–F3) that checks:

```
IF winner is numeric_code
AND no sampled values have leading zeros
THEN downgrade to integer_number
```

This is the **inverse of Rule F1**, which upgrades `postal_code`/`cpt` → `numeric_code` when leading zeros ARE present. Together they would form a **leading-zero pivot**:

```
Leading zeros present  → numeric_code   (preserve as VARCHAR)
No leading zeros       → integer_number (safe to cast BIGINT)
```

### Infrastructure Already Exists

The feature extractor (NNFT-250) already computes `has_leading_zero` (feature index 7) and the column-level aggregation computes the mean across all sampled values. The signal is available in the existing pipeline — this rule would consume it.

## Scope

Affects any pure-digit column without leading zeros where CharCNN predicts `numeric_code` at high confidence. Common in:

- Measurement columns: duration, elapsed_time, response_time
- Count columns: attendance, population, headcount, participants
- Metric columns: age, pages, score, rating (integer-valued)

In `sports_events.csv`, this also affects `attendance` (50% confidence, cross-domain hint fires but weakly).

## Status

**Not yet actioned.** Documenting as a finding for future implementation. The fix is low-risk and surgically scoped — `numeric_code` → `integer_number` only when the leading-zero signal is absent.
