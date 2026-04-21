# v12 Data Quality Audit

**Date:** 2026-04-16
**Model:** sherlock-v12 (5-branch: char+embed+stats+header+validation)
**Eval:** 204/227 (89.9% label, 92.5% domain) on 35 datasets, 227 columns
**Baseline:** sherlock-v11 (4-branch) — 204/227 (89.9% label, 93.0% domain)
**Prior art:** eval-audit-v2 (2026-04-12), collisions analysis (2026-03-26)

## Headline

v12 matches v11 on label accuracy (204/227) and drops 1 on domain accuracy (210 vs 211).
The validation branch is a neutral change overall: **4 fixed, 4 regressed, 19 persistent**.
Root cause breakdown: training data quality is the dominant issue, not model architecture.

## Bug Fix: vb.pp Prefix Mismatch

The v12 eval initially failed with `merge_bn: shape mismatch for merge_bn.running_mean, expected: [942], got: [1006]`. The overnight script swallowed the error and packaged stale v11 results.

**Root cause:** The inference crate (`finetype-model`) loaded validation branch weights under prefix `"validation"` but the training crate (`finetype-train`) saved them under prefix `"valid"`. This one-word mismatch caused the validation branch to silently disable, making the merge BN shape wrong (942 vs 1006).

**Fix:** `crates/finetype-model/src/multi_branch.rs` line 364: `vb.pp("validation")` → `vb.pp("valid")`.

## Change Summary: v11 → v12

### Fixed in v12 (4)

```
| Dataset                     | Column           | v11 Predicted  | Expected        | v11 Conf |
|-----------------------------|------------------|----------------|-----------------|----------|
| datetime_formats_extended   | eu_dot_date      | iso_8601       | dmy_dot         | 0.95     |
| finance_coverage            | cusip            | isrc           | cusip           | 0.74     |
| tech_systems                | port             | ean            | integer_number  | 0.99     |
| tech_systems                | server_hostname  | docker_ref     | hostname        | 0.99     |
```

### Regressed in v12 (4)

```
| Dataset                     | Column               | v12 Predicted      | Expected        | v12 Conf |
|-----------------------------|-----------------------|--------------------|-----------------|----------|
| datetime_formats            | year                  | compact_ym         | year            | 1.00     |
| datetime_formats_extended   | long_full_month_date  | dmy_space_full     | long_full_month | 0.74     |
| earthquakes_2024            | dmin                  | latitude           | decimal_number  | 0.97     |
| earthquakes_2024            | gap                   | amount_accounting  | decimal_number  | 0.43     |
```

### Persistent — confidence increased (6)

```
| Dataset              | Column      | Predicted        | Expected        | v11→v12 Conf  | Delta  |
|----------------------|-------------|------------------|-----------------|---------------|--------|
| tech_systems         | user_agent  | jwt              | user_agent      | 0.37→1.00     | +0.63  |
| earthquakes_2024     | depthError  | latitude         | decimal_number  | 0.61→1.00     | +0.39  |
| server_logs_json     | method      | iata_code        | http_method     | 0.29→0.61     | +0.32  |
| network_logs         | status_code | postal_code*     | integer_number  | 0.69→0.99     | +0.29  |
| codes_and_ids        | sha256      | ethereum_address | hash            | 0.96→0.97     | +0.01  |
| earthquakes_2024     | id          | geohash          | alphanumeric_id | 0.99→1.00     | +0.01  |
```

*Prediction also changed: bsb→postal_code

### Persistent — confidence stable or decreased (13)

```
| Dataset                     | Column               | Predicted        | Expected            | v11→v12 Conf | Delta  |
|-----------------------------|-----------------------|------------------|---------------------|--------------|--------|
| weather_stations_json       | location.country      | country          | country_code        | 1.00→1.00    |  0.00  |
| api_users_json              | address.country       | country          | country_code        | 1.00→1.00    |  0.00  |
| new_identity                | email_display         | email            | email_display       | 1.00→1.00    |  0.00  |
| new_technology              | data_uri              | url              | data_uri            | 1.00→1.00    |  0.00  |
| new_identity                | phone_e164            | phone_number     | phone_e164          | 1.00→1.00    |  0.00  |
| people_directory            | phone                 | ssn              | phone_number        | 1.00→1.00    |  0.00  |
| server_logs_json            | status_code           | postal_code      | integer_number      | 0.81→0.80    | -0.01  |
| ecommerce_orders            | phone                 | ssn              | phone_number        | 1.00→0.99    | -0.01  |
| multilingual                | locale                | alphanumeric_id  | locale_code         | 0.40→0.30    | -0.11  |
| representation_coverage     | scientific_notation   | plain_text*      | scientific_notation | 0.59→0.45    | -0.14  |
| new_technology              | git_sha               | ethereum_address*| hash                | 1.00→0.77    | -0.23  |
| new_geography               | geojson               | plain_text       | json                | 0.89→0.61    | -0.28  |
| network_logs                | user_agent            | url*             | user_agent          | 0.83→0.39    | -0.44  |
```

*Prediction also changed from v11

---

## Per-Item Audit (ac-01 + ac-02)

### Item 1: new_technology / data_uri

**v12 predicted:** url (1.00) | **Expected:** data_uri | **Status:** persistent

**Sample values:** `data:text/html;base64,8nOdF+...`, `data:image/png;base64,8MNKC0...`, `data:text/plain,new`

**Evidence:** All values start with `data:` scheme prefix — unambiguously data URIs, not HTTP URLs. The `data:` prefix is structurally distinct from `http://`/`https://`.

**Training data:** data_uri has 0 distilled rows, 1200 synthetic. url has 2 distilled, 1198 synthetic. Both are effectively synthetic-only. The synthetic generators produce distinct formats (`data:mime;base64,...` vs `https://domain/path`).

**Root cause:** `data_gap` — With zero real-world data_uri examples, the model treats both as URL-like patterns and picks the more common label.

---

### Item 2: new_identity / email_display

**v12 predicted:** email (1.00) | **Expected:** email_display | **Status:** persistent

**Sample values:** `Amelia Davis <amelia.davis@corp.com>`, `"Daniel Sanchez" <daniel.sanchez@company.org>`

**Evidence:** Values are RFC 5322 display name + angle-bracket email format. The email substring is present but wrapped in a display name context.

**Training data:** email_display likely has zero distilled data. email is one of the most common types in Sherlock. The model detects the email substring with high confidence and ignores the display name wrapper.

**Root cause:** `data_gap` — The model has no real-world examples of the "Name <email>" pattern to learn the distinction from bare email addresses.

---

### Item 3: new_identity / phone_e164

**v12 predicted:** phone_number (1.00) | **Expected:** phone_e164 | **Status:** persistent

**Sample values:** `+18285346333`, `+442248987091`, `+61499244691`

**Evidence:** Values are E.164 format — `+` followed by country code and subscriber digits with no separators. This is a strict subset of phone_number formatting.

**Training data:** phone_number synthetic generator also produces formats like `+1 838-291-8737` which overlap with E.164.

**Root cause:** `training_collision` — E.164 is a strict specialisation of phone_number. The training data for phone_number includes E.164-like numbers, making the boundary blurry. The model predicts the more general parent type.

---

### Item 4: weather_stations_json / location.country

**v12 predicted:** country (1.00) | **Expected:** country_code | **Status:** persistent

**Sample values:** `AU, GB, US, JP, NL, SG, CA, FR, DE, ES`

**Evidence:** Values are 2-letter ISO 3166-1 alpha-2 country codes. GT correctly labels these as country_code.

**Training data:** country has 840 distilled rows (4.5× more than country_code's 624). Critically, `geography.region.state_code` is remapped to `country_code` via label_remap.json — meaning US state abbreviations (OR, CA, FL) are trained as country codes, contaminating the label.

**Root cause:** `training_collision` — Class imbalance (4.5×) favours country, and state code contamination degrades country_code's signal.

---

### Item 5: api_users_json / address.country

**v12 predicted:** country (1.00) | **Expected:** country_code | **Status:** persistent

**Sample values:** `US, GB, FR, DE, JP, AU, CA, IN, SG, AE`

**Evidence:** Same pattern as item 4.

**Root cause:** `training_collision` — Same as item 4.

---

### Item 6: earthquakes_2024 / depthError

**v12 predicted:** latitude (1.00) | **Expected:** decimal_number | **Status:** persistent, confidence ↑ 0.61→1.00

**Sample values:** `7.431, 8.251, 11.011, 1.825, 9.663`

**Evidence:** Values are depth error measurements in km. They fall within [-90, 90] latitude range but are not geographic coordinates. Prior audit (v2 case 7) confirmed WRONG.

**Training data:** latitude has 4 distilled rows, decimal_number has 5,154. Despite 1000× imbalance favouring decimal_number, the model predicts latitude — likely because the char-level patterns and value distribution for small positive decimals trigger the latitude learned representation.

**Root cause:** `model_error` — The model over-indexes on value range for latitude without considering header context. **Confidence increase from 0.61→1.00 is alarming** — see confidence-analysis.md.

---

### Item 7: earthquakes_2024 / id

**v12 predicted:** geohash (1.00) | **Expected:** alphanumeric_id | **Status:** persistent

**Sample values:** `us6000pgkh, us6000pgkd, us6000pj75`

**Evidence:** USGS earthquake IDs with `us` prefix. Not geohashes (geohashes use base32 without semantic prefixes). Prior audit (v2 case 20) confirmed WRONG.

**Training data:** geohash has 0 distilled rows. Geohash validation `^[0-9b-hjkmnp-z]{4,12}$` actually matches these values (all chars are in base32 set, length 10 is in [4,12]). This false validation match reinforces the wrong prediction.

**Root cause:** `training_collision` — Geohash validation pattern is too permissive, matching many lowercase alphanumeric strings. Geohash has zero distilled data to learn real-world geohash distribution.

---

### Item 8: datetime_formats / year ⚠️ REGRESSED

**v12 predicted:** compact_ym (1.00) | **Expected:** year | **Status:** regressed from v11

**Sample values:** `2022, 2021, 2020, 2023`

**Evidence:** 4-digit year values. compact_ym is YYYYMM format (6 digits like `202203`). 4-digit numbers cannot be compact_ym. Prior audit (v2 case 3) confirmed WRONG.

**Training data:** Both types are likely well-represented in synthetic data. The regression from v11→v12 suggests the validation branch or retraining shifted the decision boundary. This is a clear regression — v11 classified correctly.

**Root cause:** `model_error` — New regression. The v12 model learned a worse decision boundary for year vs compact_ym. The validation branch adds no signal (neither type has validation that disambiguates 4-digit vs 6-digit values).

---

### Item 9: tech_systems / user_agent

**v12 predicted:** jwt (1.00) | **Expected:** user_agent | **Status:** persistent, confidence ↑ 0.37→1.00

**Sample values:** `Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15) AppleWebKit/537.36 ...`

**Evidence:** Full browser user agent strings. JWT tokens are base64-encoded dot-separated segments. Completely different formats. Prior audit (v2 case 6) confirmed WRONG.

**Training data:** user_agent has 1 distilled row (ALL 4 distilled rows are mislabeled — person names, product descriptions). jwt has 0 distilled rows. Both are synthetic-only. The distilled user_agent data is 100% noise.

**Root cause:** `data_gap` — Zero clean distilled examples for user_agent or jwt. **Confidence jump from 0.37→1.00 is the worst regression** — see confidence-analysis.md.

---

### Item 10: people_directory / phone

**v12 predicted:** ssn (1.00) | **Expected:** phone_number | **Status:** persistent

**Sample values:** `+61-392-253-9475, +49-954-136-4111, +33-501-574-2929`

**Evidence:** International phone numbers with country code prefix (+61/+49/+33) and dash-separated groups. SSNs don't have `+` prefix or country codes. Prior audit (v2 case 12) confirmed WRONG.

**Training data:** SSN has 0 clean distilled rows (3 total, all mislabeled with partial dates). phone_number has 2 usable distilled rows. Synthetic ssn uses NNN-NN-NNNN pattern; phone uses varied formats including dash-separated.

**Root cause:** `training_collision` — Dash-separated digit groups overlap visually between phone (3-3-4) and SSN (3-2-4). The `+` country code prefix should disambiguate but the model doesn't weight it sufficiently. SSN distilled data is 100% noise.

---

### Item 11: network_logs / status_code

**v12 predicted:** postal_code (0.99) | **Expected:** integer_number | **Status:** persistent, pred changed bsb→postal_code

**Sample values:** `400, 500, 301, 403, 503, 401, 204, 404`

**Evidence:** HTTP status codes (200-503 range). Prior audit (v2 case 10) confirmed WRONG. In v11 predicted bsb (0.69), now predicts postal_code (0.99) — different wrong answer at higher confidence.

**Training data:** postal_code has 34 distilled rows (93% noise — includes state names, "Oh No" repeated). integer_number has 5,629 distilled rows. BSB validation `^\d{3}-\d{3}$` fails for these values (no dash). The validation branch correctly eliminated bsb, but the model switched to postal_code instead of integer_number.

**Root cause:** `training_collision` — 3-digit integer ranges overlap with numeric postal codes. The validation branch helped eliminate bsb (a step forward) but the char/embed branches still route small integers to geography.address types.

---

### Item 12: ecommerce_orders / phone

**v12 predicted:** ssn (0.99) | **Expected:** phone_number | **Status:** persistent

**Sample values:** `+1-232-130-2535, +1-206-877-3615, +1-552-718-5333`

**Evidence:** US phone numbers with +1 country code and dash-separated groups. Same pattern as item 10. Prior audit (v2 case 9) confirmed WRONG.

**Root cause:** `training_collision` — Same as item 10.

---

### Item 13: codes_and_ids / sha256

**v12 predicted:** ethereum_address (0.97) | **Expected:** hash | **Status:** persistent

**Sample values:** `1abd775a8e661366c67807273a7bd0fdcd70048964cf985b4d4a1668b391dacb` (64-char hex)

**Evidence:** SHA-256 hashes (64 hex chars). Ethereum addresses are `0x` + 40 hex chars. These values have no `0x` prefix and are 64 chars, not 40. Prior audit (v2 case 2) confirmed WRONG.

**Training data:** ethereum_address has 0 distilled rows. hash has 2 distilled rows. Both synthetic-only. Hash validation `^[0-9a-f]{64}$` matches these values; ethereum validation `^0x[a-fA-F0-9]{40}$` does NOT. The validation branch provides the correct signal but other branches dominate.

**Root cause:** `model_error` — Hex string confusion. The validation branch correctly identifies these as hash (not ethereum_address) but the char/embed branches override it.

---

### Item 14: earthquakes_2024 / dmin ⚠️ REGRESSED

**v12 predicted:** latitude (0.97) | **Expected:** decimal_number | **Status:** regressed from v11

**Sample values:** `3.367, 2.417, 2.799, 1.504, 5.22`

**Evidence:** Minimum distance to seismic station in degrees. Small positive decimals in latitude range. Same confusion pattern as depthError (item 6).

**Root cause:** `model_error` — Same latitude/decimal_number confusion. New regression — v11 classified correctly. The v12 model is more aggressive at predicting latitude for small positive decimals.

---

### Item 15: server_logs_json / status_code

**v12 predicted:** postal_code (0.80) | **Expected:** integer_number | **Status:** persistent

**Sample values:** `200, 201, 204, 401, 500`

**Evidence:** Same pattern as item 11. Prior audit (v2 case 26) confirmed WRONG.

**Root cause:** `training_collision` — Same as item 11.

---

### Item 16: new_technology / git_sha

**v12 predicted:** ethereum_address (0.77) | **Expected:** hash | **Status:** persistent, pred changed tsid→ethereum_address

**Sample values:** `20ad889500783ba6609f4b95ef3af7bd53b0086f` (40-char hex)

**Evidence:** SHA-1 hashes (40 hex chars). Ethereum addresses require `0x` prefix. Prior audit (v2 case 5) rated DEBATABLE (git_sha is a valid more-specific prediction for 40-char hex). Now the model predicts ethereum_address, which is clearly wrong — `0x` prefix is absent.

**Training data:** hash validation matches both 40-char and 64-char hex. ethereum validation requires `0x` prefix, which these values lack.

**Root cause:** `model_error` — Hex string confusion between ethereum_address and hash. The prediction change from tsid (v11) to ethereum_address (v12) is a lateral move between wrong answers.

---

### Item 17: datetime_formats_extended / long_full_month_date ⚠️ REGRESSED

**v12 predicted:** dmy_space_full (0.74) | **Expected:** long_full_month | **Status:** regressed from v11

**Sample values:** `February 24, 2020`, `March 06, 2020`, `August 05, 2020`

**Evidence:** Format is `Month DD, YYYY` — full month name, zero-padded day, comma, 4-digit year. This matches `long_full_month` (American-style date with comma). `dmy_space_full` would be `DD Month YYYY` (no comma, day-first European style).

**Root cause:** `model_error` — Confusion between similar datetime formats. The comma and month-first ordering should distinguish American from European format, but the model doesn't discriminate. New regression in v12.

---

### Item 18: new_geography / geojson

**v12 predicted:** plain_text (0.61) | **Expected:** json | **Status:** persistent

**Sample values:** `{"type": "Point", "coordinates": [-29.5789, 45.6377]}`, `{"type": "Feature", "geometry": {...}}`

**Evidence:** Valid GeoJSON objects. At minimum these should be classified as JSON. Prior audit (v2 case 1) rated DEBATABLE (the model predicted the more specific geojson type). Now the model predicts plain_text — a worse answer.

**Root cause:** `model_error` — The model fails to detect JSON structure entirely, defaulting to plain_text. Confidence dropped from 0.89→0.61, suggesting increased uncertainty.

---

### Item 19: server_logs_json / method

**v12 predicted:** iata_code (0.61) | **Expected:** http_method | **Status:** persistent, confidence ↑ 0.29→0.61

**Sample values:** `GET, POST, DELETE, PUT, PATCH, OPTIONS`

**Evidence:** HTTP method strings. IATA codes are 3-letter airport identifiers (LAX, JFK). Values like GET and PUT are 3 uppercase letters matching IATA pattern. Prior audit (v2 case 22) rated DEBATABLE (categorical also reasonable).

**Training data:** http_method has no validation pattern. IATA validation `^[A-Z]{3}$` matches GET and PUT (2 of 6 values = 33% pass rate), providing a false positive signal.

**Root cause:** `training_collision` — 3-letter uppercase codes overlap between IATA and HTTP method vocabularies. IATA validation partially matches. See confidence-analysis.md for validation feature detail.

---

### Item 20: representation_coverage / scientific_notation

**v12 predicted:** plain_text (0.45) | **Expected:** scientific_notation | **Status:** persistent, pred changed decimal_number→plain_text

**Sample values:** `1.23e-4, 6.022e23, -3.14E2, 9.81e0`

**Evidence:** Standard scientific notation with e/E exponent separator. Prior audit (v2 case 31) confirmed WRONG — clearly not plain_text.

**Training data:** scientific_notation likely has limited distilled data. The `e`/`E` character in numeric context is the distinguishing feature.

**Root cause:** `data_gap` — The model has insufficient training examples to learn scientific notation patterns. Prediction degraded from decimal_number (wrong but close) to plain_text (completely wrong).

---

### Item 21: earthquakes_2024 / gap ⚠️ REGRESSED

**v12 predicted:** amount_accounting (0.43) | **Expected:** decimal_number (accepts integer_number) | **Status:** regressed from v11

**Sample values:** `109, 110, 145, 131, 185, 168, 59, 89`

**Evidence:** Integer values (azimuthal gap in degrees). GT label "decimal number" accepts both decimal_number and integer_number. The model predicts amount_accounting, which is wrong.

**Training data:** amount_accounting validation `^\(?\$?[0-9]{1,3}(,[0-9]{3})*(\.[0-9]{1,2})?\)?$` matches some of these values (3-digit numbers without formatting). This false validation match may contribute.

**Root cause:** `model_error` — New regression. The v12 model confuses plain integers with accounting amounts. Low confidence (0.43) suggests high uncertainty.

---

### Item 22: network_logs / user_agent

**v12 predicted:** url (0.39) | **Expected:** user_agent | **Status:** persistent, pred changed docker_ref→url

**Sample values:** `Mozilla/5.0 (Macintosh; Intel Mac OS X) Chrome/88.0`, `Mozilla/5.0 (Windows NT 10.0) Chrome/96.0`

**Evidence:** Abbreviated user agent strings. Same root cause as item 9. The prediction changed from docker_ref (v11) to url (v12) — different wrong answer. Low confidence (0.39) shows high model uncertainty.

**Root cause:** `data_gap` — Same as item 9. Zero clean distilled user_agent data.

---

### Item 23: multilingual / locale

**v12 predicted:** alphanumeric_id (0.30) | **Expected:** locale_code | **Status:** persistent

**Sample values:** `de-DE, ja-JP, pt-BR`

**Evidence:** BCP 47 locale tags (language-COUNTRY). Only 3 distinct values. Low cardinality makes it look like a generic identifier.

**Training data:** locale_code likely has limited distilled data. The hyphenated 2+2 letter pattern is shared with many identifier formats.

**Root cause:** `data_gap` — locale_code has insufficient training data for the model to learn the specific BCP 47 pattern. Lowest confidence in the eval (0.30).

---

## Root Cause Distribution (ac-02)

```
| Root Cause          | Count | Items                                                          |
|---------------------|-------|----------------------------------------------------------------|
| model_error         | 9     | 6, 8, 13, 14, 16, 17, 18, 20, 21                              |
| training_collision  | 8     | 3, 4, 5, 7, 10, 11, 12, 15                                    |
| data_gap            | 6     | 1, 2, 9, 19, 22, 23                                           |
| gt_error            | 0     | —                                                              |
| generator_defect    | 0     | —                                                              |
```

**Key finding:** All 23 misclassifications are genuine model or training data issues. Zero GT errors remain — the prior audit (v2) already corrected GT labels.

## Training Data Inspection: Regression Pairs (ac-03)

The 4 regressions involve distinct confusion patterns. Training data samples for each:

### Regression 1: year → compact_ym (item 8)

Year values are 4-digit (`2022`). compact_ym is 6-digit YYYYMM (`202203`). Synthetic generators produce distinct formats. This regression suggests the v12 retraining shifted the year/compact_ym boundary — the validation branch provides no signal (neither type has length-discriminating validation). No training data overlap.

### Regression 2: long_full_month → dmy_space_full (item 17)

Both are full-month-name date formats. `long_full_month` = `Month DD, YYYY` (comma). `dmy_space_full` = `DD Month YYYY` (no comma, day-first). Synthetic generators produce distinct formats but both share the "full month name + day + year" character-level pattern. The comma position and field ordering are the discriminators. Training data overlap is moderate — both types share the same character vocabulary.

### Regression 3: decimal_number → latitude (item 14, also persistent item 6)

Latitude synthetic range is [-90, 90]. decimal_number distilled data includes 5,154 rows spanning wide ranges but many values in [-90, 90] (ages, percentages, measurements). The model learned to associate small positive decimals with latitude more aggressively in v12. Training data overlap is severe in the [-90, 90] range.

### Regression 4: decimal_number → amount_accounting (item 21)

Gap values are 3-digit integers. amount_accounting validation `^\(?\$?[0-9]{1,3}(,[0-9]{3})*(\.[0-9]{1,2})?\)?$` matches unformatted 1-3 digit integers. The amount_accounting generator produces formatted values (`$1,234.56`) that shouldn't overlap with plain integers, but the validation pattern is too permissive for unformatted values.

## Ground Truth Corrections (ac-05)

**No GT corrections required.** All 23 misclassifications are genuine model/training data issues. The prior audit (v2, 2026-04-12) already corrected GT labels and expanded interchangeability rules. The current eval/schema_mapping.yaml is accurate.

## Adjusted v12 Score (ac-06)

Since no GT corrections are needed, the score stands:

```
| Metric           | v12 (original) | v12 (adjusted) | v11 (baseline) |
|------------------|----------------|----------------|----------------|
| Label accuracy   | 204/227 (89.9%)| 204/227 (89.9%)| 204/227 (89.9%)|
| Domain accuracy  | 210/227 (92.5%)| 210/227 (92.5%)| 211/227 (93.0%)|
| Actionability    | 99.8%          | 99.8%          | 99.8%          |
```

The validation branch is net-neutral on label accuracy and causes a 1-point domain regression.
