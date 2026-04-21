# v13 Misclassification Audit

**Date:** 2026-04-17
**Model:** sherlock-v13
**Eval result:** 212/227 (93.4% label, 93.8% domain)
**Misclassifications:** 15

## Summary Table

```
| # | Dataset              | Column           | Predicted          | Expected          | Conf | Root Cause            | Fix Approach     |
|---|----------------------|------------------|--------------------|-------------------|------|-----------------------|------------------|
| 1 | new_technology       | data_uri         | url                | data_uri          | 1.00 | hierarchical_subtype  | retrain          |
| 2 | new_identity         | email_display    | email              | email_display     | 1.00 | hierarchical_subtype  | retrain          |
| 3 | new_identity         | phone_e164       | phone_number       | phone_e164        | 1.00 | hierarchical_subtype  | retrain          |
| 4 | weather_stations_json| location.country | country            | country_code      | 1.00 | ground_truth_debate   | GT fix           |
| 5 | api_users_json       | address.country  | country            | country_code      | 1.00 | ground_truth_debate   | GT fix           |
| 6 | datetime_formats     | year             | compact_ym         | year              | 1.00 | model_error           | retrain          |
| 7 | tech_systems         | user_agent       | jwt                | user_agent        | 1.00 | model_error           | retrain          |
| 8 | earthquakes_2024     | gap              | amount_accounting  | decimal_number    | 0.96 | model_error           | retrain          |
| 9 | earthquakes_2024     | depthError       | latitude           | decimal_number    | 0.92 | model_error           | retrain          |
|10 | server_logs_json     | status_code      | postal_code        | integer_number    | 0.76 | training_collision    | retrain + rule   |
|11 | earthquakes_2024     | id               | username           | alphanumeric_id   | 0.53 | training_collision    | retrain          |
|12 | new_geography        | geojson          | plain_text         | json              | 0.52 | data_gap              | retrain          |
|13 | codes_and_ids        | sha256           | tsid               | hash              | 0.49 | training_collision    | retrain          |
|14 | server_logs_json     | user_agent       | plain_text         | user_agent        | 0.46 | data_gap              | retrain          |
|15 | network_logs         | status_code      | postal_code        | integer_number    | 0.43 | training_collision    | retrain + rule   |
```

## Root Cause Distribution

```
| Root Cause           | Count | Items                          |
|----------------------|-------|--------------------------------|
| hierarchical_subtype | 3     | #1, #2, #3                     |
| model_error          | 4     | #6, #7, #8, #9                 |
| training_collision   | 4     | #10, #11, #13, #15             |
| ground_truth_debate  | 2     | #4, #5                         |
| data_gap             | 2     | #12, #14                       |
```

---

## Detailed Analysis

### 1. new_technology / data_uri: url vs data_uri (1.00)

**Root cause: hierarchical_subtype**

Sample values:
```
data:text/html;base64,8nOdF+3VDWSnq51mZKDOacAE5L+Ujr79HqI/0Cf4usexv13pXbjG...
data:image/png;base64,8MNKC0Kf3Ugux+QWike9QAIljMzsxbUOFWXlsckGFK1G0e0SSM1Ifs=
data:text/plain,new
data:text/html;base64,9cexE03QEjoB1ruDo3MdFldsEUAhu+iixEO=
data:image/png,golf
```

Data URIs (`data:...`) are a sub-scheme of URIs. The model sees these as URLs because they share the `scheme:` prefix pattern. The header hint makes it worse: "data_uri" contains "uri" which triggers the hardcoded `url` hint, reinforcing the model's wrong prediction at 1.00 confidence.

**Fix:** Retrain with more data_uri examples. Additionally, the header hint `h.contains("uri")` is too aggressive -- "data_uri" should not map to `url`. But per decision 0042, regex header hints are deprecated. The model's header branch (Model2Vec) needs to learn this distinction. A validation-based value_sharpen rule checking for `data:` prefix could also work as a short-term fix.

### 2. new_identity / email_display: email vs email_display (1.00)

**Root cause: hierarchical_subtype**

Sample values:
```
Amelia Davis <amelia.davis@corp.com>
Amelia Lee <amelia.lee@example.com>
"Daniel Sanchez" <daniel.sanchez@company.org>
Hannah Lewis <hannah.lewis@mail.com>
Samuel Miller <samuel.miller@example.com>
```

`email_display` is the RFC 5322 display format: `Name <email>`. The model sees the `@` sign and email pattern within and classifies as plain `email`. This is a parent/sibling confusion -- the model lacks training signal for the wrapping `Name <...>` pattern.

**Fix:** Retrain with more email_display training examples. A value_sharpen rule could detect the `Name <email>` pattern, but retraining is preferred (decision 0038).

### 3. new_identity / phone_e164: phone_number vs phone_e164 (1.00)

**Root cause: hierarchical_subtype**

Sample values:
```
+18285346333
+824504224114
+442248987091
+8649585610936
+16687493438
```

E.164 format phones start with `+` followed by digits (no spaces, dashes, or parentheses). The model sees phone patterns and classifies as the broader `phone_number`. The distinction is purely formatting: E.164 has no separators.

**Fix:** Retrain with explicit E.164 examples. A value_sharpen rule checking `^\+\d{7,15}$` (no separators) could disambiguate, but retraining is preferred.

### 4. weather_stations_json / location.country: country vs country_code (1.00)

**Root cause: ground_truth_debate**

Sample values:
```
AU, GB, US, JP, NL, CA, DE, ES, FR, IT, SG
```

These are 2-letter ISO 3166-1 alpha-2 codes. The GT label is `country_code`, but the model predicts `country`. The data IS country codes, so the GT label is correct and the model is wrong.

However, the header path `location.country` triggers the hardcoded hint `country` (exact match on "country"). The hint overrides because the model's prediction IS `country` at 1.00 -- the hint and model agree. But both are wrong.

The JSON field path `location.country` is ambiguous -- it says "country" but the values are codes. This is a genuine conflict between header semantics and value semantics.

**Fix:** GT fix is tempting but incorrect -- these really are country codes. The model needs to learn that 2-letter uppercase strings are `country_code` not `country`. The header hint for "country" fires and overrides. Short-term: add "country code" variants to header hint. Long-term: retrain to distinguish codes from names.

**Reclassification:** On reflection, this is a **training_collision** compounded by a **feature_gap** (header hint "country" fires incorrectly). Changing GT would be wrong. The fix is: (a) retrain with better country_code vs country separation, and (b) a value_sharpen rule that checks if all values are 2-3 letter uppercase strings and reclassifies to country_code.

### 5. api_users_json / address.country: country vs country_code (1.00)

**Root cause: ground_truth_debate (same as #4)**

Sample values:
```
US, GB, FR, DE, JP, AE, AU, CA, IN, IT, KR, NL, SE, SG
```

Identical pattern to #4. 2-letter ISO 3166-1 alpha-2 codes labeled as `country_code` in GT. Model + header hint both say `country`.

**Fix:** Same as #4. These are a pair.

### 6. datetime_formats / year: compact_ym vs year (1.00)

**Root cause: model_error**

Sample values:
```
2022, 2021, 2021, 2022, 2020, 2023, 2022, 2021, 2021, 2020
```

Pure 4-digit years. The model predicts `compact_ym` (YYYYMM format) at 1.00 confidence. This is clearly wrong -- YYYYMM would be 6 digits like "202201". The hardcoded header hint for "year" maps to `datetime.component.year` and would override, but the same-domain threshold is 0.95 and the model's confidence is 1.00, so the hint is blocked.

The model is highly confident but wrong. This suggests a training data issue -- the `compact_ym` training examples may include 4-digit values, or the char patterns overlap too much.

**Fix:** Retrain. Ensure compact_ym training data is exclusively 6-digit YYYYMM values. Consider lowering the same-domain header override threshold from 0.95 to 0.98 or adding a value_sharpen rule: if all values are 4-digit numbers in 1900-2100, override to `year`.

### 7. tech_systems / user_agent: jwt vs user_agent (1.00)

**Root cause: model_error**

Sample values:
```
Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/100.0.2578.378 Safari/537.36
Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/99.0.744.225 Safari/537.36
```

These are clearly browser user agent strings. The model predicts JWT at 1.00 confidence. Both types are long strings with dots and slashes, but JWTs have a very distinctive `xxxxx.xxxxx.xxxxx` three-part base64 structure. This is a severe model error.

The header "user_agent" does not trigger any hardcoded hint (no `h.contains("user_agent")` pattern). There's no semantic hint firing either.

**Fix:** Retrain with better user_agent examples. Add a value_sharpen rule: if values match `Mozilla/` prefix or standard UA patterns, override to user_agent. Also consider adding "user agent" to the hardcoded header hints as a stopgap.

### 8. earthquakes_2024 / gap: amount_accounting vs decimal_number (0.96)

**Root cause: model_error**

Sample values:
```
109, 110, 145, 131, 185, 168, 59, 89, 189, 219
```

These are plain integers representing azimuthal gap in degrees. 14,126 of 14,132 values are integers; only 6 have decimal points. The model predicts `amount_accounting` (formatted currency like `$1,234.56`). These values have no currency symbols, commas, or decimal formatting. This is a clear model error.

The header "gap" does not trigger any hint.

**Fix:** Retrain. The model needs to learn that bare integers without currency formatting are not accounting amounts. A feature_sharpen rule could check: if no currency symbols and no comma-separated thousands, it's not amount_accounting.

### 9. earthquakes_2024 / depthError: latitude vs decimal_number (0.92)

**Root cause: model_error**

Sample values:
```
7.431, 8.251, 11.011, 1.825, 1.969, 1.962, 1.799, 9.663, 6.117, 13.707
```

Range: 0.09 to 31.96, mean 4.93. These are depth error margins in km. The values fall within the latitude range (-90 to 90), so the model confuses them with latitude. However, the header "depthError" does not trigger the latitude hint (only "latitude" and "lat" do).

The header "depthError" contains "Error" which has no matching hint. The model should use the absence of a geographic header + the fact that these are all positive values to avoid the latitude prediction.

**Fix:** Retrain. The model should learn that `depthError` column context doesn't match geography. A header hint for "error" -> decimal_number could help but is too broad.

### 10. server_logs_json / status_code: postal_code vs integer_number (0.76)

**Root cause: training_collision**

Sample values:
```
200, 201, 202, 204, 301, 401, 403, 413, 500
```

3-digit integers. The model predicts `postal_code` because 3-digit numbers overlap with postal code patterns (some countries have 3-digit postal codes). The GT maps `http status code` -> `integer_number` in schema_mapping.yaml.

Only 25 records in this dataset, and status codes are a small set of 3-digit integers. The header "status_code" contains "code" but there's no generic hint for that. There's no hint for "status" either.

**Fix:** Retrain with more 3-digit integer examples that aren't postal codes. A value_sharpen rule could check: if the column header contains "status" and values are 3-digit numbers from the HTTP status code set (1xx-5xx), override to integer_number. Or add a header hint for "status code" -> integer_number.

### 11. earthquakes_2024 / id: username vs alphanumeric_id (0.53)

**Root cause: training_collision**

Sample values:
```
us6000pgkh, us6000pgkd, us6000pj75, us6000pj76, us6000pgkb
```

All 10 characters, pattern: 2 letters + 8 alphanumeric. These look like usernames AND alphanumeric IDs -- both are short alphanumeric strings. Confidence is low (0.53), showing the model is uncertain.

The "id" header hint was removed (decision 0034) because ID columns are genuinely ambiguous.

**Fix:** Retrain with more alphanumeric_id examples that have this prefix+code pattern. The low confidence suggests the model sees both possibilities but narrowly picks the wrong one.

### 12. new_geography / geojson: plain_text vs json (0.52)

**Root cause: data_gap**

Sample values:
```
{"type": "Point", "coordinates": [-29.5789, 45.6377]}
{"type": "Point", "coordinates": [-173.0126, -64.6871]}
{"type": "Feature", "geometry": {"type": "Point", "coordinates": [150.8264, 36.9448]}, "properties": {}}
```

These are JSON objects. The model predicts `plain_text` at 0.52 confidence. The GT label `geojson` maps to `container.object.json` in schema_mapping.yaml. The model fails to recognize structured JSON content, possibly because JSON strings are uncommon as CSV column values.

Average value length is 68 characters -- short enough that the JSON structure may not dominate the character-level features.

**Fix:** Retrain with more JSON-in-CSV examples. The `json` type likely has very few training examples since JSON is usually a container format, not a column value.

### 13. codes_and_ids / sha256: tsid vs hash (0.49)

**Root cause: training_collision**

Sample values:
```
1abd775a8e661366c67807273a7bd0fdcd70048964cf985b4d4a1668b391dacb
00d98d1078437455dbbfd3e652312396cc31affe717f8691b773cdb8da3f9fe4
```

All 64-character hex strings. TSIDs in the training data are 32-character hex strings. Both are hex-encoded identifiers. The model sees hex patterns and picks TSID over hash. Confidence is near 50% (0.49), showing maximum uncertainty.

The header "sha256" does not trigger a hint.

**Fix:** Retrain. Ensure hash training data includes 64-char SHA-256 specifically, distinct from 32-char TSIDs/MD5. A value_sharpen rule checking hex string length (64 = SHA-256, 40 = SHA-1, 32 = MD5/TSID) could disambiguate. Or a feature_sharpen rule on string length statistics.

### 14. server_logs_json / user_agent: plain_text vs user_agent (0.46)

**Root cause: data_gap**

Sample values:
```
Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36
PostmanRuntime/7.36.1
curl/8.4.0
kube-probe/1.28
python-requests/2.31.0
```

Only 25 records. Mix of full browser UAs and short tool UAs. The model predicts `plain_text` at 0.46 -- very uncertain. Small sample size (25 values vs typical 100) likely hurts the statistical features.

This is the same column type that #7 (tech_systems) also gets wrong but with a different wrong answer (jwt vs plain_text). The small sample size here is the key difference.

**Fix:** Retrain with more diverse user_agent examples including short tool UAs. The 25-record dataset limits what the multi-branch model can extract from value statistics.

### 15. network_logs / status_code: postal_code vs integer_number (0.43)

**Root cause: training_collision (same as #10)**

Sample values:
```
200, 201, 204, 301, 400, 401, 403, 404, 500, 502, 503
```

Identical pattern to #10. 3-digit HTTP status codes misclassified as postal codes. 100 records (vs 25 in #10), yet confidence is even lower (0.43 vs 0.76), suggesting more uncertainty with more data.

**Fix:** Same as #10.

---

## Recommendations by Fix Approach

### 1. Retrain (addresses 13 of 15)

All 15 misclassifications would benefit from retraining. Priority focus areas:

**High priority (model errors with 1.00 confidence):**
- **compact_ym vs year (#6):** Ensure compact_ym training data is strictly 6-digit YYYYMM. 4-digit years must not appear in compact_ym training examples.
- **jwt vs user_agent (#7):** User agent training examples need more diversity. JWT and UA have very different structure -- this should be learnable.
- **amount_accounting vs decimal_number (#8):** Accounting amounts need currency formatting features. Bare integers should not match.
- **latitude vs decimal_number (#9):** Positive-only decimal values outside geographic context should not trigger latitude.

**Medium priority (hierarchical subtypes):**
- **url vs data_uri (#1):** Add more `data:` scheme examples to data_uri training. Ensure url training doesn't include data URIs.
- **email vs email_display (#2):** Add `Name <email>` format examples. Decontaminate email training data.
- **phone_number vs phone_e164 (#3):** Add strict E.164 format examples (no separators).

**Lower priority (collisions at low confidence):**
- **postal_code vs integer_number (#10, #15):** Separate 3-digit status codes from postal code training data.
- **username vs alphanumeric_id (#11):** Better alphanumeric_id coverage for prefix+code patterns.
- **tsid vs hash (#13):** Separate by hex string length (32 vs 64 chars).
- **plain_text vs json (#12):** Add JSON-as-string training examples.
- **plain_text vs user_agent (#14):** More diverse UA training including short tool strings.

### 2. Ground Truth Fixes (addresses 2 of 15)

**Re-evaluation needed for #4 and #5:**

Both `weather_stations_json / location.country` and `api_users_json / address.country` contain 2-letter ISO codes (AU, GB, US) with GT label `country_code`. The model predicts `country` because the JSON field name is "country" and the header hint for "country" fires.

The GT label IS correct -- these are country codes, not country names. The issue is that the header path says "country" but the values are codes. Two options:

- **Option A: Accept as model error.** The model should learn that 2-letter uppercase strings are codes even when the header says "country". Keep GT as-is.
- **Option B: Fix eval mapping.** Accept `country` as a close match for `country_code` in the schema mapping (add to `finetype_labels` list). Debatable since they're genuinely different types.

**Recommendation:** Option A. These are genuine misclassifications. The model needs to weight value evidence over header evidence for this case. Alternatively, a value_sharpen rule could check: if predicted=country but all values are 2-3 char uppercase, override to country_code.

### 3. New Rules (addresses 4 of 15, as supplementary fixes)

Per decision 0038, rules are a last resort. But some patterns are deterministic:

- **R-new-1: HTTP status codes.** If header contains "status" and values are 3-digit numbers in {1xx-5xx}, override to integer_number. Addresses #10, #15.
- **R-new-2: Country code detection.** If predicted=country but all values match `^[A-Z]{2,3}$`, override to country_code. Addresses #4, #5.
- **R-new-3: Hash length disambiguation.** If predicted=tsid but string length consistently >= 40, override to hash. Addresses #13.

These are recommended as training reinforcement ONLY if retrain doesn't fix them.

### 4. Header Hint Additions (addresses 1 of 15, stopgap)

Per decision 0042, hardcoded header hints are deprecated. However:

- The "data_uri" header triggers `url` hint via `h.contains("uri")` -- this actively hurts. If any header hint fix is warranted, it's making the "uri" substring match more specific (e.g., require standalone "uri" not inside "data_uri"). But this contradicts decision 0042.

**Recommendation:** Do not add new header hints. Fix through retraining.

---

## Verdict: Is a v14 Retrain Warranted?

**Yes, but not urgently.**

The 15 misclassifications break down as:

- **4 model errors at 1.00 confidence** (#6, #7, #8, #9) -- these indicate training data quality issues that are fixable with targeted data curation.
- **3 hierarchical subtypes at 1.00 confidence** (#1, #2, #3) -- these need dedicated subtype training data. The model has not learned the parent-child distinction.
- **4 training collisions at low confidence** (#10, #11, #13, #15) -- the model is already uncertain; better training data balance would flip these.
- **2 data gaps** (#12, #14) -- new training examples needed for json-in-csv and diverse user agents.
- **2 GT/mapping debates** (#4, #5) -- country vs country_code. Fixable by either approach.

**v14 retrain priority tiers:**

1. **Training data curation** (fixes #6, #7, #8, #9): Audit compact_ym, user_agent, amount_accounting, latitude training data for contamination or pattern overlap.
2. **Subtype decontamination** (fixes #1, #2, #3): Ensure url doesn't include data URIs, email doesn't include display format, phone_number doesn't include pure E.164.
3. **Collision separation** (fixes #10, #11, #13, #15): Better negative mining for postal_code vs status codes, username vs alphanumeric_id, tsid vs hash.
4. **Data gap filling** (fixes #12, #14): Add json-in-csv and diverse UA training examples.
5. **Country code rule** (fixes #4, #5): Either retrain or add value_sharpen rule R-new-2.

**Expected impact:** A well-executed v14 retrain addressing tiers 1-4 could fix 10-13 of the 15 misclassifications, pushing accuracy to 222-225/227 (97.8-99.1%).

**Recommendation:** Proceed with v14 when sprint capacity allows. The remaining 15 misclassifications are well-understood and the fixes are concrete. No architecture changes needed.
