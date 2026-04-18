# Actionability Misclassification Audit

**Date:** 2026-04-18
**Model:** sherlock-v14 (218/227, 96.0% label)
**Scope:** 10 columns with <95% actionability success rate

---

## Executive Summary

The 10 actionability failures are **misclassifications, not format_string gaps**. All affected types have format_strings defined in the taxonomy. The failures occur because the predicted type's format_string can't parse the actual data — because the prediction is wrong.

Three root causes:

```
| Root Cause                              | Columns | Fix Path                        |
|-----------------------------------------|---------|---------------------------------|
| "date/timestamp" header hint catch-all  | 7       | Narrow or remove hint (code)    |
| Version ↔ date format confusion         | 2       | Disambiguation rule or retrain  |
| Compound value (blood pressure)         | 1       | Ground truth / taxonomy gap     |
```

One of the 7 header hint columns (ecommerce_orders_json/order_date) is actually a **correct prediction** — the actionability failure is an eval pipeline bug.

**Zero overlap** with the 9 label-accuracy errors. Combined true error surface: **18 misclassified columns** + 1 eval bug.

---

## Category 1: Header Hint "date/timestamp" Catch-All (7 columns)

**Root cause:** Line 4181 in `column.rs`:
```rust
if (h.contains("date") || h.contains("timestamp") || h.contains("datetime"))
    && !h.contains("month")
{
    return Some("datetime.timestamp.iso_8601");
}
```

Any header containing "date" or "timestamp" is blanket-hinted to iso_8601, regardless of the actual data format.

```
| Column                                   | Actual Values          | Correct Type | Model Prediction | Override Mechanism            |
|------------------------------------------|------------------------|--------------|------------------|-------------------------------|
| datetime_formats/us_date                 | 07/26/2022             | mdy_slash    | mdy_slash (63%)  | hardcoded: h.contains("date") |
| datetime_formats/eu_date                 | 26/07/2022             | dmy_slash    | (similar)        | hardcoded: h.contains("date") |
| multilingual/date                        | 11.05.2020             | dmy_dot      | (similar)        | hardcoded: h.contains("date") |
| datetime_formats_ext/american_timestamp  | 02/24/2020 05:12 PM   | mdy_12h      | mdy_12h          | same-category: "timestamp"    |
| datetime_formats_ext/european_timestamp  | 24/02/2020 17:12      | dmy_hm       | dmy_hm           | same-category: "timestamp"    |
| datetime_coverage/clf_timestamp          | [15/Mar/2024:10:30…]  | clf          | clf (43.8%)      | same-category: "timestamp"    |
| ecommerce_orders_json/order_date         | 2024-03-15T09:23:41Z  | iso_8601 ✓   | iso_8601 ✓       | EVAL BUG (see below)          |
```

### Why the model is right

Tested `us_date` values via `infer --mode column`:
- **Without header:** model predicts `mdy_slash` at 63.3% confidence → **correct**
- **With header "us_date":** header hint overrides to `iso_8601` via `h.contains("date")`

Tested `clf_timestamp` values:
- **With header "clf_timestamp":** model's vote distribution shows `clf` at 43.8% → **correct**
- But `h.contains("timestamp")` → iso_8601, same-category override fires

**The model already knows these formats.** The header hint system destroys correct predictions.

### Eval pipeline bug: ecommerce_orders_json/order_date

Prediction is correct (iso_8601) and format_string matches (`%Y-%m-%dT%H:%M:%SZ`). But actionability shows 0% because:

1. `read_json_auto()` auto-detects `order_date` as TIMESTAMP type
2. `CAST("order_date" AS VARCHAR)` produces `2024-03-15 09:23:41` (no T, no Z)
3. `TRY_STRPTIME('2024-03-15 09:23:41', '%Y-%m-%dT%H:%M:%SZ')` → NULL (no match)

**Fix:** Use `read_json_auto(path, all_varchar=true)` in `eval_actionability.rs` line 28.

### Why these aren't in the 9 label errors

Two masking mechanisms:

1. **Timestamp interchangeability rule** (`matching.rs` L17-19): `is_label_match` treats ALL `datetime.timestamp.*` subtypes as interchangeable. So `iso_8601` predicted for `mdy_12h`, `dmy_hm`, `clf` counts as "correct" in label accuracy. This is correct at the "is this a timestamp?" level but wrong for actionability.
   - Affects: american_timestamp, european_timestamp, clf_timestamp

2. **Partial match quality excluded** (`eval_report.rs` L119-127): gt_labels with `match_quality: partial` are excluded from accuracy calculation entirely.
   - Affects: us_date, eu_date, multilingual/date (gt_label="date" → partial)

---

## Category 2: Version ↔ Date Format Confusion (2 columns)

```
| Column                        | Values           | Predicted       | Correct Type                 | Confidence |
|-------------------------------|------------------|-----------------|------------------------------|------------|
| codes_and_ids/semantic_version| 0.2.53, 6.27.84  | dmy_short_dot  | technology.development.version| 0.9998     |
| tech_systems/version          | 10.6.2, 9.1.11   | dmy_short_dot  | technology.development.version| 0.9982     |
```

**Root cause:** The model confuses `X.Y.Z` version strings with `DD.MM.YY` short dates. Both have the pattern `digits.digits.digits`. High confidence because dmy_short_dot is a valid pattern match.

**Model test (5 samples):** Without header, model predicts `decimal_number` (49.8% confidence). Neither `version` nor `dmy_short_dot`. With 80 samples (full dataset), the model converges on `dmy_short_dot`.

**Not counted in label accuracy:** gt_label "version" has `match_quality: partial` → excluded from accuracy calculation.

### Disambiguation signal

Version strings have these distinguishing features:
- First segment can exceed 31 (e.g., `10.6.2` → day=10 is valid, but many versions have first segment >31)
- Third segment can exceed 31 (e.g., `0.2.53` → 53 is not a valid 2-digit year)
- Values like `6.27.84` → 84 could be a valid year but 27 exceeds any month interpretation

A value-based rule could check: if ≥80% of `X.Y.Z` values have any segment >31, override `dmy_short_dot` → `version`.

---

## Category 3: Compound Value (1 column)

```
| Column                        | Values     | Predicted       | GT Label |
|-------------------------------|------------|-----------------|----------|
| medical_records/blood_pressure| 155/82     | decimal_number  | value    |
```

**Root cause:** Blood pressure readings (systolic/diastolic) are compound values in `NNN/NN` format. The model predicts `decimal_number` — wrong, but there's no correct FineType type either.

**Not counted in label accuracy:** gt_label "value" has `match_quality: partial` → excluded. Among partial candidates for "value", `decimal_number` IS listed as acceptable.

**Actionability failure:** `CAST('155/82' AS DOUBLE)` → NULL. The `/` makes it unparseable as a number.

**Options:**
- Accept as out-of-scope (compound medical values are domain-specific)
- Update ground truth to "code" or remove from eval
- Add `identity.medical.blood_pressure` type (low priority — single dataset)

---

## Combined Error Surface

```
| Source              | Count | Details                                        |
|---------------------|-------|------------------------------------------------|
| Label-accuracy errors (reported)    | 9  | In report misclassifications table      |
| Header hint overrides (masked)      | 6  | 3 via timestamp interchangeability,     |
|                                     |    | 3 via partial match quality exclusion   |
| Version/date confusion (excluded)   | 2  | Partial match quality for "version"     |
| Compound value (excluded)           | 1  | Partial match quality for "value"       |
| Eval pipeline bug                   | 1  | JSON auto-type loses format             |
| **True misclassifications**         |**18**|                                        |
| **Eval bugs**                       |**1**|                                        |
```

**True accuracy (if all errors counted):** ~209/228 ≈ 91.7%
The reported 218/227 (96.0%) is inflated by three mechanisms: partial exclusion, timestamp interchangeability, and the eval bug.

---

## Recommendations

### R1: Fix eval pipeline JSON bug (quick win)

Change `eval_actionability.rs` line 28:
```rust
// Before:
format!("read_json_auto('{escaped}')")
// After:
format!("read_json_auto('{escaped}', all_varchar=true)")
```

Effect: ecommerce_orders_json/order_date goes from 0% → ~100% actionability. Net: 1 false alarm removed.

### R2: Narrow "date/timestamp" header hint (6 columns fixed)

The blanket `h.contains("date") || h.contains("timestamp")` is the single most harmful header hint remaining. Three options:

**Option A: Remove entirely.** Let the model decide all date/timestamp formats. The model already gets 6 of these 7 correct without the hint. Risk: some "date" headers where the model defaults to a generic type might regress. Need to check the 18 "helped" columns from the header hint analysis.

**Option B: Add same-category confidence guard.** In `apply_header_sharpen`, when the hint is `iso_8601` and the model already predicts a SPECIFIC `datetime.timestamp.*` or `datetime.date.*` subtype with confidence >0.3, don't override. This preserves the hint for columns where the model is lost (predicts plain_text/categorical) while protecting correct specific predictions.

**Option C: Change hint from iso_8601 to domain-level.** Instead of hinting a SPECIFIC type (iso_8601), hint the DOMAIN (datetime). Let the model's prediction within the datetime domain stand. This requires architectural change to the hint system.

**Recommendation:** Option B — lowest risk, preserves hint benefits, fixes the 6 broken columns.

### R3: Version ↔ date disambiguation rule (2 columns fixed)

Add R31 in `value_sharpen()`:
- If label is `datetime.date.dmy_short_dot`
- And ≥80% of values matching `X.Y.Z` have any segment >31 (impossible day/year)
- Override to `technology.development.version`

Alternative: retrain with version examples that overlap with date format.

### R4: Tighten timestamp interchangeability (eval accuracy)

The `is_label_match` rule treating all `datetime.timestamp.*` as interchangeable masks 3 real errors. Consider splitting into "format-compatible" groups:
- ISO-family: iso_8601, iso_8601_milliseconds, iso_8601_microseconds, iso_8601_offset, rfc_3339
- Regional: mdy_12h, dmy_hm, mdy_hms
- Specialized: clf, syslog_bsd, rfc_2822

This would surface american_timestamp, european_timestamp, and clf_timestamp as label errors, giving a more honest accuracy picture.

### R5: Blood pressure — accept or remove from eval

Not a model error — no FineType type exists. Recommend updating manifest ground truth from "value" to a custom note that excludes it from eval scoring.

---

## Priority Order

```
| Priority | Fix                              | Impact              | Effort   |
|----------|----------------------------------|---------------------|----------|
| P1       | R1: Eval JSON bug                | 1 false alarm fixed | 5 min    |
| P2       | R2: Date/timestamp hint guard    | 6 columns fixed     | 30 min   |
| P3       | R3: Version disambiguation       | 2 columns fixed     | 30 min   |
| P4       | R4: Tighten interchangeability   | Honest reporting     | 1 hour   |
| P5       | R5: Blood pressure eval fix      | 1 false alarm fixed | 5 min    |
```

After P1–P3: expected actionability at ≥95% across all columns. After P4: label accuracy drops from 218/227 to ~215/230 (more honest but lower headline number).

---

## Overlap with 9 Label-Accuracy Errors

The 9 label errors from the eval report are entirely separate columns:

```
| Column                        | Predicted            | Expected         | Root Cause     |
|-------------------------------|----------------------|------------------|----------------|
| tech_systems/user_agent       | jwt                  | user_agent       | model_error    |
| ecommerce_orders/phone        | ssn                  | phone_number     | model_error    |
| people_directory/phone        | ssn                  | phone_number     | model_error    |
| network_logs/user_agent       | whitespace_separated | user_agent       | model_error    |
| server_logs_json/method       | iata_code            | http_method      | model_error    |
| earthquakes_2024/id           | username             | alphanumeric_id  | model_error    |
| tech_systems/server_hostname  | plain_text           | hostname         | model_error    |
| new_geography/geojson         | dms                  | json             | model_error    |
| multilingual/locale           | entity_name          | locale_code      | model_error    |
```

All 9 are genuine model errors — no header hint involvement, no eval masking. These need retraining or new disambiguation rules.

Combined with the 6 header hint errors and 2 version errors: **17 true misclassifications** that could be fixed (plus 2 eval bugs and 1 ground truth gap).
