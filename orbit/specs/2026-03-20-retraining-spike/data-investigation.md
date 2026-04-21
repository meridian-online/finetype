# Data Quality Investigation: 85K vs 58K Distilled Rows

**Date:** 2026-03-20
**Source:** output/distillation-v3/sherlock_distilled.csv.gz (85,194 rows)

## Three-Way Split

| Category | Rows | % | Individual Values |
|----------|------|---|-------------------|
| Qualifying (≥5 values) | 58,424 | 68.6% | 644,030 |
| Sparse (<5 values) | 26,163 | 30.7% | 72,139 |
| JSON parse errors | 599 | 0.7% | — |
| Empty final_label | 8 | 0.0% | — |
| **Total** | **85,194** | | **716,169** |

## Agreement Rate Comparison

| Group | Agreement | Disagreement | Agreement Rate |
|-------|-----------|--------------|----------------|
| Qualifying (≥5) | 16,350 | 42,074 | 28.0% |
| Sparse (<5) | 7,726 | 18,437 | 29.5% |

Agreement rates are nearly identical. No evidence that sparse rows have lower label quality.

## Confidence Distribution

| Confidence | Qualifying | % | Sparse | % |
|------------|-----------|---|--------|---|
| High | 31,972 | 54.7% | 10,553 | 40.3% |
| Medium | 24,202 | 41.4% | 13,267 | 50.7% |
| Low | 2,250 | 3.9% | 2,343 | 9.0% |

Sparse rows skew slightly toward medium/low confidence. 9% low-confidence in sparse vs 3.9% in qualifying. This is expected — less data means less certainty.

## Type Coverage

| Category | Types |
|----------|-------|
| Qualifying only | 47 |
| Sparse only | 21 |
| Both | 103 |
| **Total distilled** | **171** |

The 21 sparse-only types are all very low count (1–5 rows each, mostly 1 row). They include: `swift_bic` (3), `cpt` (3), `urn` (3), `ordinal` (5), `amount_minor_int` (2), `isbn` (2), `ssn` (2), `short_dmy` (2), and 13 types with 1 row each. Nearly all are disagreement rows.

## Value Count Distribution

| Values/column | Rows | Cumulative % |
|---------------|------|-------------|
| 0 | 300 | 0.4% |
| 1 | 388 | 0.8% |
| 2 | 10,754 | 13.5% |
| 3 | 8,641 | 23.7% |
| 4 | 6,080 | 30.9% |
| 5 | 10,475 | 43.2% |
| 10 | 16,232 | 62.3% |
| 20 | 12,254 | 76.7% |

## Value Explosion Impact

| Scenario | Columns | Individual Values | Notes |
|----------|---------|-------------------|-------|
| Qualifying only (≥5) | 58,424 | 644,030 | Strong per-column signal |
| All rows | 84,587 | 716,169 | +11% values, +21 types |
| Difference | +26,163 | +72,139 | Marginal gain |

## Recommendation: Use qualifying rows only (≥5 values)

**Rationale:**
1. **Minimal value gain:** Sparse rows add 72K values (11% increase) — modest for a training set that will also include ~375K synthetic values.
2. **Slightly lower confidence:** 9% low-confidence in sparse vs 4% in qualifying. More uncertain labels.
3. **Negligible type gain:** 21 sparse-only types all have 1–5 rows. At value explosion, this is 1–15 training samples per type — noise, not signal.
4. **Cleaner experiment:** Using qualifying rows keeps the data preparation simpler and more defensible. If the spike succeeds, sparse rows can be added in a follow-up.

The 58K qualifying rows explode to ~644K individual training values across 150 types. This is a substantial dataset — nearly double the current synthetic training set (372K).
