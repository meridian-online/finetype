# v13 Retrain Brief (ac-07)

**Date:** 2026-04-16
**Input:** v12 data quality audit — 23 misclassifications, 0 GT errors
**Goal:** Actionable fixes for v13 training to reduce misclassifications from 23 to ≤15

## Priority 1: Distilled Data Decontamination

**Impact: 8 items (phone/ssn, country/country_code, status_code/postal_code)**

The distilled data from Sherlock has severe contamination for several types:

### 1a. Remove state_code → country_code remap

The `data/label_remap.json` maps `geography.region.state_code` to `country_code`. This means US state abbreviations (OR, CA, FL, NC) are trained as country codes, directly confusing country vs country_code.

**Fix:** Remove the state_code→country_code remap. Either: (a) drop state_code rows entirely, or (b) create a dedicated `state_code` type in the taxonomy. Country_code should only train on actual ISO 3166-1 alpha-2 codes.

**Expected impact:** Fix items 4, 5 (country→country_code).

### 1b. Filter noise from SSN distilled data

SSN has 3 distilled rows, ALL mislabeled (contain partial dates like `"-- --, 1918"`). Phone_number has 4 distilled rows, 2 of which are year ranges (`"1985 - 1987"`).

**Fix:** Drop all distilled rows for SSN (zero clean examples). Filter phone_number distilled rows to keep only those containing actual phone-format values. The types will be synthetic-only, which is fine — their synthetic generators produce well-differentiated patterns.

**Expected impact:** Reduce phone/ssn confusion (items 10, 12).

### 1c. Filter noise from user_agent distilled data

User_agent has 4 distilled rows, ALL mislabeled (person names, product descriptions, loan statuses). Zero actual user agent strings.

**Fix:** Drop all distilled rows for user_agent. The synthetic generator produces good user agent strings.

**Expected impact:** Fix items 9, 22 (user_agent misclassification).

### 1d. Filter noise from postal_code distilled data

Postal_code has 34 distilled rows, ~93% noise (state names, "Oh No" repeated, "Zoom 8"). Only ~4 rows contain actual postal codes.

**Fix:** Filter to keep only rows where values match numeric postal code patterns or known alphanumeric formats.

**Expected impact:** Reduce status_code/postal_code confusion (items 11, 15).

## Priority 2: Validation Pattern Gaps

**Impact: 6 items (method/iata, depthError/latitude, user_agent/jwt)**

The confidence analysis revealed that 3 types lack validation patterns, preventing the validation branch from providing correct signal:

### 2a. Add http_method validation

Pattern: `^(GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS|CONNECT|TRACE)$`

This gives 100% pass rate for HTTP methods vs 33% for IATA's `^[A-Z]{3}$`. The validation branch would have strong discriminative signal.

**Expected impact:** Fix item 19 (method→iata_code).

### 2b. Add user_agent validation

Pattern: Must contain one of: `Mozilla/`, `curl/`, `python-requests/`, `Wget/`, `Go-http-client/`, `axios/`, `PostmanRuntime/`, `kube-probe/`, or other known prefixes.

**Expected impact:** Help fix items 9, 22 by giving the validation branch positive signal.

### 2c. Add latitude range validation

Pattern: `^-?([0-8]?\d(\.\d+)?|90(\.0+)?)$` (values in [-90, 90] range)

This is tricky — many decimal numbers are in this range. But combined with header context and other features, it provides useful signal. Consider whether this is net-positive.

**Expected impact:** Uncertain. May not help since decimal_number values also fall in this range. Consider this optional.

### 2d. Tighten geohash validation

Current pattern `^[0-9b-hjkmnp-z]{4,12}$` is too permissive — matches many lowercase alphanumeric strings.

Options:
- Add minimum length requirement (≥6 chars for reasonable precision)
- Add character distribution check (geohashes use all base32 chars, not biased toward letters or digits)
- Add a negative: exclude strings with obvious non-geohash patterns (e.g., country code prefix + digits like `us6000`)

**Expected impact:** Help fix item 7 (id→geohash).

## Priority 3: Class Balance Adjustments

**Impact: 3-4 items (latitude/decimal_number, country/country_code)**

### 3a. Cap distilled-to-synthetic ratio for over-represented types

Types with >1000 distilled rows (country: 3,488, decimal_number: 5,154, integer_number: 5,629) dominate the training mix. The 70/30 blend ratio only applies when distilled data exists — for types with <5 distilled rows, training is 99.5%+ synthetic.

**Fix:** Cap distilled rows per type at 600 (50% of the 1,200 total per type). Excess distilled data creates class imbalance that overwhelms synthetic data for rare types.

**Expected impact:** Reduce latitude/decimal_number confusion (items 6, 14) and country/country_code imbalance (items 4, 5).

### 3b. Hard-negative mining for latitude vs decimal_number

Generate synthetic decimal_number examples specifically in the [-90, 90] range with non-geographic headers (e.g., "error", "score", "depth"). This teaches the model that small decimals aren't always latitude.

**Expected impact:** Fix items 6, 14 and prevent regression of item 14.

## Priority 4: Merge Layer Rebalancing

**Impact: systemic**

The confidence analysis shows the validation branch provides correct signal in 4/6 cases but is overridden by char/embed branches. The merge layer's batch normalization may be underweighting validation features.

### 4a. Increase validation branch hidden dimensions

Current: 239 → 128 → 64. The char branch is 960 → 450 → 450 — 7× larger. Increase validation branch to 239 → 192 → 128 to give it more representational capacity.

### 4b. Monitor validation branch gradient flow

Add training diagnostics to report per-branch gradient norms. If the validation branch gradients are vanishing relative to char/embed, consider a branch-weighted loss or gradient scaling.

**Expected impact:** Systemic improvement in validation branch influence.

## Summary: Expected Impact

```
| Priority | Fixes                                 | Items                    | Effort |
|----------|---------------------------------------|--------------------------|--------|
| P1       | Distilled data decontamination        | 4, 5, 9, 10, 11, 12, 15, 22 | Low  |
| P2       | Validation pattern gaps               | 7, 19, (9, 22)           | Low    |
| P3       | Class balance + hard negatives        | 4, 5, 6, 14              | Medium |
| P4       | Merge layer rebalancing               | systemic                 | High   |
```

**Conservative estimate:** P1+P2 alone should fix 6-8 items, moving from 204/227 to 210-212/227 (92-93%). P3 adds another 2-3. P4 is higher-risk architectural change.

**Items unlikely to be fixed by retraining alone:**
- Item 3 (phone_e164/phone_number) — inherent type hierarchy overlap
- Item 1 (data_uri/url) — requires more diverse data_uri examples
- Item 23 (locale) — low cardinality makes classification unreliable
- Item 18 (geojson) — may need structural JSON detection, not char-level
- Item 17 (long_full_month/dmy_space_full) — subtle datetime disambiguation

## Next Steps

This brief feeds into the next `/orb:design` session for v13 training. The P1 fixes (distilled data decontamination) should be implemented immediately as they are low-risk, high-confidence improvements. P2 (validation patterns) can be added to the taxonomy in the same session. P3/P4 require a separate design decision.
