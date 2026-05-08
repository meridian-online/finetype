# Sharpen Rule Set Audit

**Date:** 2026-04-17
**Auditor:** Nightingale (Claude)
**Model baseline:** sherlock-v13 (212/227, 93.4% label accuracy)
**Reference:** v13 misclassification audit (15 items), decisions 0038, 0042

## Purpose

Rationalise the Sharpen rule set before v14 retrain. Per decision 0038, rules are a last resort — if improved training data makes a rule redundant, it should go. Per decision 0042, regex header hints are deprecated in favour of learned approaches.

---

## Summary Table

```
| ID    | Rule                              | Verdict  | Reason                                                                |
|-------|-----------------------------------|----------|-----------------------------------------------------------------------|
| F1    | Leading-zero → numeric_code       | KEEP     | Deterministic signal model cannot learn (leading zeros lost in embed)  |
| F2    | Slash segments → docker_ref       | DEFER    | Model may learn this with v14 data; keep as safety net until then     |
| F3    | HS code digit/dot pattern         | REMOVE   | Superseded by R20 (hs_code validation gate), double-fires redundantly |
| F4    | git_sha → hash (removed)          | N/A      | Already removed in code                                               |
| F5    | numeric_code → integer/decimal    | KEEP     | Essential counterbalance to F1; prevents over-triggering              |
| F6    | Short alpha → categorical         | DEFER    | Model may learn this; low risk to defer                               |
| R1    | Slash date disambiguation         | KEEP     | Deterministic — model fundamentally cannot distinguish mdy/dmy        |
| R2    | Short date disambiguation         | KEEP     | Same as R1, for short date formats                                    |
| R3    | Coordinate disambiguation         | KEEP     | Deterministic range check — lat [-90,90] vs lon [-180,180]            |
| R4    | IPv4 detection                    | DEFER    | Model may learn dotted-quad; deterministic check is cheap insurance   |
| R5    | Day-of-week detection             | KEEP     | Deterministic vocabulary check, 100% precision                        |
| R6    | Month name detection              | KEEP     | Deterministic vocabulary check, 100% precision                        |
| R7    | Boolean subtype normalization     | KEEP     | Model cannot distinguish binary/terms/initials — value-level check    |
| R8    | Gender detection                  | DEFER    | Model may learn this; deterministic check is cheap insurance          |
| R9    | Boolean override (non-boolean)    | KEEP     | Catches multi-value integer columns misclassified as boolean          |
| R10   | Small integer ordinal             | SIMPLIFY | Useful but over-complex; simplify trigger conditions                  |
| R11   | Categorical detection             | SIMPLIFY | Overlaps with R15 (attractor cardinality); merge into single rule     |
| R12   | Numeric type disambiguation       | KEEP     | Year/increment/postal detection from value ranges — model can't learn |
| R13   | SI number override                | KEEP     | Deterministic suffix check, no SI suffix → not SI number              |
| R14   | Duration vs SEDOL override        | KEEP     | Deterministic 'P' prefix check, ISO 8601 pattern                     |
| R15   | Attractor demotion                | KEEP     | Core safety net — validation + confidence + cardinality signals       |
| R16   | Text length demotion              | DEFER    | Model may learn address vs text length; median check is cheap         |
| R17   | UTC offset override               | KEEP     | Deterministic [+-]HH:MM pattern — model can't distinguish from time  |
| R19   | Percentage without % → decimal    | KEEP     | Deterministic — no % sign means not a percentage                      |
| R20   | HS code validation gate           | KEEP     | Deterministic format check prevents F3 false positives                |
| R21   | Coordinate plausibility gate      | KEEP     | Prevents lat/lon on out-of-range decimals. Directly relevant to #8/#9 |
| R22   | UPC digit-count gate              | KEEP     | Deterministic 12-digit check, catches EAN/NPI confusion               |
| R23   | ISIN format gate                  | KEEP     | Deterministic format check for ISRC→ISIN correction                   |
| R24   | ISSN/EIN dash-position gate       | KEEP     | Deterministic dash position for ISSN vs EIN                           |
| EPOCH | detect_epoch_seconds              | KEEP     | Deterministic range check; model can't learn 10-digit epoch pattern   |
| HH-*  | Hardcoded header hints (exact)    | SIMPLIFY | Too many entries; audit for removable entries below                    |
| HH-k  | Hardcoded header hints (keyword)  | SIMPLIFY | Several cause misclassifications — audit below                        |
| HS-*  | apply_header_sharpen override     | SIMPLIFY | Complex multi-path logic; reduce confidence threshold paths           |
```

## Verdict Counts

```
| Verdict  | Count |
|----------|-------|
| KEEP     | 19    |
| REMOVE   | 1     |
| DEFER    | 5     |
| SIMPLIFY | 5     |
| N/A      | 1     |
```

---

## Detailed Analysis

### Feature Sharpen Rules (F1-F6)

#### F1: Leading-zero → numeric_code [KEEP]

**What it does:** When >=30% of values have leading zeros and the model predicts postal_code or CPT, override to numeric_code. Leading zeros are a strong signal for code-like data that should be preserved as VARCHAR.

**Why keep:** Leading zeros are stripped during embedding and numeric feature extraction. The model fundamentally cannot see this signal — it's a property of the string representation that vanishes in the feature space. F1 is deterministic and high-precision.

**Eval impact:** No misclassifications caused. Correctly fires on zero-padded identifier columns.

#### F2: Slash segments → docker_ref [DEFER]

**What it does:** When model predicts hostname but mean slash segment count >= 1.5, override to docker_ref. Docker refs like "docker.io/library/nginx:latest" have slashes; hostnames don't.

**Why defer:** The multi-branch model includes a stats branch with SEGMENT_COUNT_SLASH as a feature. With v14 training data including docker_ref examples, the model should learn this distinction. The rule is a good safety net but may become redundant.

**Test after v14:** If the model correctly classifies docker_ref columns without F2 firing, remove.

#### F3: HS code digit/dot pattern [REMOVE]

**What it does:** When model predicts decimal_number, checks digit_ratio >= 0.75 and dot_segments >= 2.0 (or path B with float fraction < 1.0) to override to hs_code. Has negative-prefix and dot-variance guards.

**Why remove:** R20 (HS code validation gate) is a stronger, more specific check. R20 validates actual HS code format (4+ digits with dot-separated 2-digit groups), while F3 uses loose statistical features that produce false positives on columns like pe_ratio, humidity_pct, and sepal_length. The F3→R20 sequence means F3 triggers false hs_code predictions that R20 then has to clean up. Removing F3 eliminates this round-trip.

**Risk:** Low. R20 remains as the definitive HS code check. The model's stats branch sees the same features F3 uses.

#### F5: numeric_code without leading zeros → integer/decimal [KEEP]

**What it does:** When model predicts numeric_code but leading_zero_ratio < 0.01, override to integer_number (or decimal_number if >50% floats). Essential counterbalance to F1.

**Why keep:** F1 aggressively reclassifies to numeric_code based on leading zeros. When the model itself predicts numeric_code on non-zero-padded data, F5 corrects it. Without F5, false numeric_code predictions would persist.

#### F6: Short alpha → categorical [KEEP as DEFER]

**What it does:** When model predicts file.extension but values have mean length <= 4.0, dot segments < 1.1, and alpha ratio >= 0.8, override to categorical.

**Why defer:** Short alphabetic codes (e.g., "USD", "EUR", "GBP") look like file extensions to the model. The multi-branch model's header branch should learn this distinction with better training data. However, the rule has low false-positive risk.

**Test after v14:** If the model correctly classifies short-code columns without F6 firing, remove.

---

### Value Sharpen Rules (R1-R24)

#### R1: Slash date disambiguation (mdy_slash vs dmy_slash) [KEEP]

**What it does:** When model predicts either slash date type, examines values: if any first component > 12, it's DMY; if any second component > 12, it's MDY.

**Why keep:** This is fundamentally ambiguous — "03/04/2024" is valid in both formats. The model cannot learn this distinction; only value-level evidence disambiguates. Deterministic and 100% precise when evidence exists.

#### R2: Short date disambiguation (short_mdy vs short_dmy) [KEEP]

**Why keep:** Same reasoning as R1, for 2-digit year formats.

#### R3: Coordinate disambiguation (latitude vs longitude) [KEEP]

**What it does:** Checks value ranges: all values in [-90, 90] → latitude; any values outside [-90, 90] but within [-180, 180] → longitude.

**Why keep:** Latitude and longitude values overlap in the [-90, 90] range. The model sees identical numeric distributions. Only the actual value range disambiguates.

#### R4: IPv4 detection [DEFER]

**What it does:** Unconditionally checks all columns for dotted-quad IPv4 pattern (X.X.X.X where each octet 0-255). Fires regardless of model prediction.

**Why defer:** The multi-branch model has a char branch and stats branch that should learn the dotted-quad pattern. However, R4 fires unconditionally (not gated on model prediction), which means it catches cases where the model predicts something entirely wrong. This makes it a strong safety net.

**Test after v14:** Check how often R4 overrides a non-IP prediction. If never, remove.

#### R5: Day-of-week detection [KEEP]

**What it does:** If >=80% of values match a known day name vocabulary, classify as day_of_week.

**Why keep:** Day names are a finite, well-defined set. The model sometimes confuses them with categorical or first_name. The deterministic vocabulary check is 100% precise.

#### R6: Month name detection [KEEP]

**Why keep:** Same reasoning as R5, for month names.

#### R7: Boolean subtype normalization [KEEP]

**What it does:** Determines the correct boolean sub-type (binary 0/1, terms true/false, initials T/F) by examining actual values.

**Why keep:** The three boolean sub-types have identical semantics but different representations. The model would need to learn a three-way split on very simple patterns. The value check is trivial and 100% precise. Also catches boolean-valued columns that the model misclassifies as categorical.

#### R8: Gender detection [DEFER]

**What it does:** If ALL values match a known gender vocabulary (male/female/M/F/non-binary/etc.), classify as gender.

**Why defer:** The model should learn gender columns from training data. The vocabulary check is conservative (ALL values must match). However, gender detection from values alone is very reliable and the model already gets 100% on gender in eval.

**Observation:** Gender is already 3/3 (100%) in eval. R8 may be firing and helping, or the model may be doing it alone. Need instrumentation to determine.

#### R9: Boolean override [KEEP]

**What it does:** When model predicts boolean but values have >2 unique integers with spread > 1, override to integer_number. Also catches single-char non-boolean categoricals.

**Why keep:** The model over-predicts boolean on small-integer columns (e.g., SibSp: 0-8). This rule distinguishes true boolean (exactly 2 values) from count/ordinal data. Deterministic and high-precision.

#### R10: Small integer ordinal [SIMPLIFY]

**What it does:** When model predicts day_of_month/integer/increment and values are small positive integers with 2-10 unique values and max <= 20, override to ordinal.

**Why simplify:** The trigger conditions (misfit_types check, n_unique range, min/max bounds, repetition check) are complex. Consider:
1. Merging with R11 (categorical detection) since ordinal is a kind of categorical.
2. Simplifying the trigger to just "model predicts numeric-adjacent type + low cardinality + repetition."

#### R11: Categorical detection [SIMPLIFY]

**What it does:** Two sub-rules: (a) all single-char non-digit values with >2 unique → categorical, (b) 3-20 unique short string values when model predicts a generic type → categorical.

**Why simplify:** Overlaps significantly with R15's cardinality signal (attractor demotion Signal 3). R15 already checks 1-20 unique values for text attractors. Consider merging R11's categorical detection into R15's cardinality branch.

#### R12: Numeric type disambiguation [KEEP]

**What it does:** Complex value-range analysis: year detection (4-digit in 1900-2100), sequential increment detection, postal code detection (consistent digit length, typical range).

**Why keep:** Year, increment, and postal code are all integer-valued types with overlapping value distributions. The model sees similar numeric features for all of them. Range and pattern analysis is the only reliable disambiguator. The year detection in particular is critical for audit item #6 (compact_ym vs year).

**Note:** R12's year detection is the defense against #6, but it currently doesn't fire because the model predicts compact_ym (not in R12's trigger set). Consider expanding the numeric_types trigger list to include compact_ym.

#### R13: SI number override [KEEP]

**What it does:** When model predicts si_number, checks if any values have SI suffixes (K/M/B/T/G). If not, override to decimal_number.

**Why keep:** Deterministic — if no SI suffix exists in the values, they're not SI numbers. Simple and high-precision.

#### R14: Duration vs SEDOL override [KEEP]

**What it does:** When model predicts SEDOL, checks if >=50% of values start with 'P' followed by duration component letters. If so, override to ISO 8601 duration.

**Why keep:** ISO 8601 durations and SEDOL codes are both 5-8 char alphanumeric strings starting with 'P'. The duration component letters (Y/M/D/T/H/S) after 'P' are the only distinguisher. Deterministic.

#### R15: Attractor demotion [KEEP]

**What it does:** The most complex rule — three signals: (1) validation failure >50% → demote, (2) confidence < 0.85 without validation → demote, (3) text attractor with 1-20 unique values → categorical.

**Why keep:** This is the core safety net against the model's known failure mode: over-confidently assigning specific types to generic data. postal_code is the primary numeric attractor; first_name, phone_number, username, street_name are text attractors; icao_code, ndc, cusip, top_level_domain are code attractors. Validation-based demotion is the strongest rule in the entire set.

**Observation:** Directly relevant to audit items #10 and #15 (postal_code on status codes). postal_code is a NUMERIC_ATTRACTOR, so R15 should fire. The fact that postal_code predictions at 0.76 and 0.43 survive suggests the validation check is passing (3-digit integers match the postal code regex). This means the postal_code validator is too permissive for these cases.

#### R16: Text length demotion [DEFER]

**What it does:** When model predicts full_address but median value length > 100 chars, override to plain_text.

**Why defer:** A crude but effective heuristic. The multi-branch stats branch includes LENGTH features, so the model should learn that very long text isn't an address. However, the model got 3/3 on full_address in eval, so R16 is not actively needed right now.

#### R17: UTC offset override [KEEP]

**What it does:** When >=80% of values match [+-]HH:MM pattern (6 chars), override to UTC offset.

**Why keep:** UTC offsets share the HH:MM digit pattern with time types. The mandatory leading +/- sign is the only distinguisher. Model gets utc 2/2 in eval — but this may be because R17 fires. Deterministic and high-precision.

#### R19: Percentage without % → decimal [KEEP]

**What it does:** When model predicts percentage but no values contain '%', override to decimal_number.

**Why keep:** A percentage without a percentage sign is just a decimal number. Deterministic, simple, and obviously correct.

#### R20: HS code validation gate [KEEP]

**What it does:** When model predicts hs_code, checks if >=50% of values match HS code format (4+ digits with dot-separated groups). If not, demote to decimal_number.

**Why keep:** This is the definitive HS code check. Prevents false hs_code predictions on decimal columns. Supersedes the looser F3 statistical check.

#### R21: Coordinate plausibility gate [KEEP]

**What it does:** When model predicts latitude or longitude, checks if >10% of values exceed [-180, 180]. If so, demote to decimal_number.

**Why keep:** Directly addresses audit items #8 (gap: 219 max value) and #9 (depthError). Values outside all possible coordinate ranges prove they're not coordinates. However, this gate currently doesn't fire on #8 and #9 because the values in those columns ARE within [-180, 180] range. For #8 (gap), values go up to 219 which exceeds 180, so R21 SHOULD fire. For #9 (depthError), all values are 0.09-31.96, well within [-180, 180], so R21 correctly doesn't fire.

**Status on audit items:** R21 should help with #8 but the model predicts amount_accounting (not latitude), so R21's coordinate trigger doesn't activate. R21 can't help with #9 because the values are in-range.

#### R22: UPC digit-count gate [KEEP]

**What it does:** When model predicts UPC, checks if >=50% of values are 12 digits. If not, checks for EAN (13/8 digits) or falls back to numeric_code.

**Why keep:** Deterministic digit-length check. UPC is exactly 12 digits; EAN is 13 or 8. The model confuses these because they're all digit strings of similar length.

#### R23: ISIN format gate [KEEP]

**What it does:** When model predicts ISRC, checks if >=50% of values match ISIN format (2-letter country code + 9 alphanumeric + 1 check digit). If so, reclassify as ISIN.

**Why keep:** Deterministic format check. ISIN and ISRC have different structures (ISIN: 12 chars, letter+alnum; ISRC: CC-XXX-YY-NNNNN with dashes).

#### R24: ISSN/EIN dash-position gate [KEEP]

**What it does:** When model predicts EIN, checks if >=50% of values match ISSN format (DDDD-DDDD, dash at position 4). If so, reclassify as ISSN.

**Why keep:** Deterministic dash-position check. EIN has dash at position 2; ISSN at position 4.

#### EPOCH: detect_epoch_seconds [KEEP]

**What it does:** Checks if 80%+ of values are 10-digit integers in Unix epoch range (2000-2050) or 13-digit in millisecond range.

**Why keep:** Epoch timestamps are 10-digit integers that the model consistently misclassifies as NPI or other identity types. The range check is deterministic and high-precision.

---

### Hardcoded Header Hints

#### Exact-match hints (header_hint match block, ~180 entries)

**General assessment:** The exact-match block maps specific header strings to types. Most are unambiguous ("email" → email, "latitude" → latitude). These are deprecated per decision 0042 but currently necessary because the Model2Vec semantic hint doesn't cover all cases.

##### Actively harmful entries:

1. **`"url" | "uri"` in exact match → url** — "uri" as an exact match is fine, but the KEYWORD match `h.contains("uri")` at line 4135 matches "data_uri" headers, causing audit item #1. **REMOVE** the keyword match (keep the exact match).

2. **`"country" | "country name"` → country** — Fires on JSON paths like "location.country" where the values are actually country codes. Causes audit items #4 and #5. The model also predicts country, so both model and hint agree on the wrong answer. **SIMPLIFY** — the exact match is fine, but the v14 spec's AC-05 adds a post-hint country_code guard to fix this.

##### Entries the model likely handles:

These exact-match entries map to types the model already classifies correctly at high confidence. After v14, consider removing them one-by-one with eval verification:

- `"email"` → email (model gets 4/5 on email, 1 miss is email_display subtype)
- `"uuid" | "guid"` → uuid (model gets 3/3)
- `"latitude" | "lat"` → latitude (model gets 5/6, 1 miss is depthError)
- `"longitude" | "lng" | "lon" | "long"` → longitude (model gets 5/5)
- `"city" | "city name"` → city (model gets 5/5)
- `"state" | "province" | "region"` → state (model gets 5/5)

##### Entries that actively help:

- `"gender" | "sex"` → gender — Unambiguous, but model also gets 3/3
- `"age"` → integer_number — Type removed in v0.5.2; redirect prevents model from trying to classify as non-existent type
- `"epoch" | "unix *"` entries → epoch types — Critical; model misclassifies epochs as NPI
- `"altitude" | "elevation"` → integer_number — Prevents coordinate misclassification
- Various scientific measurement keywords — Prevent latitude confusion on numeric data
- `"publisher" | "company" | ...` → entity_name — Prevent city confusion on entity columns
- `"author"` → full_name — Unambiguous person name
- `"survived" | "alive" | ...` → binary — Unambiguous boolean columns
- `"class" | "pclass" | "grade" | "rank"` → ordinal — Prevent day_of_month confusion

#### Keyword/substring hints (h.contains blocks, ~40 entries)

**General assessment:** These are more dangerous than exact matches because they match substrings. Several cause false positives.

##### Actively harmful:

1. **`h.contains("uri")` → url** (line 4135) — Matches "data_uri" → url, causing audit item #1. **REMOVE** this keyword match entirely. The exact match for "uri" at line 3859 already covers the intended case.

2. **`h.contains("country")` implicit through exact match** — Not a keyword match, but the exact "country" match hits JSON paths. Addressed by AC-05 guard.

##### Potentially over-broad:

3. **`h.contains("name") && h.ends_with(" name")` → full_name** (line 4061) — "display name", "account name" → full_name is fine, but "company name", "product name", "file name" would be wrong. Guard against datetime components exists, but no guard against non-person entity names.

4. **`h.contains("address") && !email && !ip && !mac && !bitcoin && !crypto && !wallet` → full_address** (line 4073) — Exclusion list is growing. "memory address", "register address" would be false positives.

5. **`h.contains("date") || h.contains("timestamp") || h.contains("datetime")` → iso_8601** (line 4115) — Very broad. "update_date", "create_date" are fine, but "mandate", "candidate" contain "date" as substring. The normalization replaces `_` with space, so "mandate" would have `h = "mandate"` which does contain "date".

6. **`h.contains("link")` within exact match → url** (line 3859) — "chain_link", "cufflink" would be false positives. Low risk in practice.

##### Safe to keep:

- `h.contains("email")` → email — Very specific
- `h.contains("phone") || h.contains("tel") || h.contains("mobile")` → phone — Specific
- `h.contains("zip") || h.contains("postal")` → postal_code — Specific
- `h.contains("born") || h.contains("birth") || h.contains("dob")` → iso_date — Specific
- `h.contains("password")` → password — Unambiguous
- `h.contains("price") || h.contains("cost") || h.contains("salary") ...` → amount — Specific

---

### apply_header_sharpen Override Logic

**General assessment:** The override function has 7 distinct paths with varying confidence thresholds. This is complex but each path addresses a real case.

1. **Measurement disambiguation (height/weight)** [KEEP] — Deterministic swap within measurement types.
2. **Scientific measurement override** [KEEP] — Coordinates → decimal when header says measurement. Directly relevant to audit #9.
3. **Person-name vs location protection** [KEEP] — Prevents Model2Vec "name" hints from overriding location predictions.
4. **Same-domain geographic override** [KEEP] — When both hint and prediction are location types, trust header.
5. **Same-category hardcoded override** [SIMPLIFY] — Unconditional for hardcoded hints. This is what makes "country" hints override country_code evidence. AC-05 adds a guard.
6. **Cross-domain hardcoded override** [SIMPLIFY] — Overrides across domains. Potential for damage if a keyword hint fires incorrectly.
7. **Hardcoded hint authority (threshold-based)** [SIMPLIFY] — Same-domain 0.95, cross-domain 0.85. The 0.95 threshold blocks the "year" header hint from overriding compact_ym at 1.00 confidence (audit #6).

**Key finding:** The 0.95 same-domain threshold was raised from 0.90 to unblock phone@0.915 overriding SSN. But it now blocks year@1.00 overriding compact_ym. If v14 training fixes compact_ym, this threshold is fine. If not, the threshold would need to be raised further or a specific compact_ym guard added.

---

## Rules That Actively Cause Misclassifications

```
| Audit # | Rule                              | Impact                                          |
|---------|-----------------------------------|------------------------------------------------|
| #1      | h.contains("uri") → url           | "data_uri" header → url, blocks correct model   |
| #4, #5  | "country" exact match → country   | JSON path "location.country" → country on codes  |
| #6      | 0.95 same-domain threshold        | Blocks "year" hint from overriding compact_ym    |
```

The first is directly harmful (keyword match too broad). The second is addressed by AC-05. The third is a threshold trade-off (raising it would fix #6 but might break other cases).

---

## Recommended Removal Order

### Phase 1: Safe removals (pre-v14, no risk)

1. **Remove F3** (HS code feature rule). R20 validation gate is the proper check. F3 creates false hs_code predictions that R20 must then clean up. Net effect: fewer false hs_code intermediate states.

2. **Remove `h.contains("uri")` keyword match** (line 4135). The exact match for "uri" at line 3859 covers the intended case. The keyword match is the only one that actively causes a misclassification in the eval set. This is not adding a new rule — it's removing a harmful substring match while keeping the exact match.

### Phase 2: Post-v14 removals (test-then-remove)

3. **Remove F2** (docker_ref slash segments) — test if v14 model classifies docker_ref without it.
4. **Remove F6** (short alpha categorical) — test if v14 model handles file.extension distinction.
5. **Remove R4** (IPv4 detection) — test if v14 model learns dotted-quad pattern.
6. **Remove R8** (gender detection) — test if v14 model classifies gender columns without it.
7. **Remove R16** (text length demotion) — test if v14 model distinguishes full_address from plain_text.

### Phase 3: Simplifications (post-v14, with care)

8. **Merge R10 and R11** — Combine small-integer ordinal and categorical detection into a single low-cardinality rule.
9. **Simplify header hint keyword matches** — Remove over-broad patterns: `h.contains("date")` (matches "mandate"), `h.ends_with(" name")` (matches non-person names).
10. **Simplify apply_header_sharpen paths** — The 7 override paths could be reduced to 3-4 by merging same-category and cross-domain hardcoded paths with clearer precedence.

---

## Impact on v14 Spec

### ACs that change if rules are removed:

**If F3 is removed (Phase 1):**
- No AC change needed. F3 removal is independent of training data changes. R20 remains.
- Minor positive: one fewer feature_sharpen path to debug during eval analysis.

**If `h.contains("uri")` is removed (Phase 1):**
- **AC-05 impact:** None. AC-05 is about country_code, not URI.
- **Audit item #1 impact:** Removing the URI keyword match means the model's prediction stands uncontested by a bad hint. If v14 retraining fixes the model (AC-01 subtype decontamination), #1 is fixed. If not, the model's wrong prediction stands — but at least the header hint isn't making it worse.
- **Risk:** If another column has a header like "download_uri" that the model misclassifies, the keyword match would have rescued it. Mitigated by the exact-match "uri" still existing.

**If Phase 2 removals happen:**
- Each removal must be tested against the 227-column eval. AC-08 verification step already requires column-level regression diff. Add a sub-item: "For each removed rule, verify no regression on columns that rule previously corrected."

### New AC recommended:

Consider adding to the v14 spec:

```
ac-09: Remove h.contains("uri") keyword match from header_hint() at line 4135.
       Keep the exact match for "uri" at line 3859. Verify no regression on
       url columns in eval set. This directly addresses audit item #1.
```

And:

```
ac-10: Remove F3 (HS code feature rule) from feature_sharpen(). R20 (HS code
       validation gate) in value_sharpen is the definitive check. Verify hs_code
       eval column still correct.
```

---

## Appendix: Rule Firing Estimate

Without instrumentation, we cannot know exactly which rules fire during eval. Based on code analysis and eval results, the following rules are likely firing on the 227-column eval:

- **R1/R2** (date disambiguation): On the 2+ slash-date and short-date columns
- **R3** (coordinate): On the 6 latitude + 5 longitude columns  
- **R5/R6** (day/month names): On the 1 day_of_week + 1 month_name columns
- **R7** (boolean subtype): On the 4 boolean.terms + 1 binary columns
- **R12** (numeric): On year, increment, postal_code columns
- **R15** (attractor demotion): On columns where model is uncertain about attractor types
- **R17** (UTC offset): On the 2 UTC offset columns
- **R21** (coordinate plausibility): Possibly on earthquake columns
- **EPOCH**: On epoch timestamp columns
- **Header hints**: On most columns with recognizable headers

**Recommendation:** Add a `--verbose-rules` flag to `finetype profile` that logs which rules fire per column, enabling data-driven rule retirement.
