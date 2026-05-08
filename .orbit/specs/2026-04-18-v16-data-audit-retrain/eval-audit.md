# Eval Ground Truth Audit — ac-01

**Date:** 2026-04-18
**Auditor:** Nightingale (Claude agent)
**Scope:** All 338 ground truth labels in `eval/datasets/manifest.csv`
**Datasets:** 35 (22 CSV, 4 JSON/NDJSON, 9 coverage/expansion CSVs)

## Methodology

For every (dataset, column, gt_label) triple in manifest.csv:

1. Read 5+ sample values from the actual data file
2. Verify the gt_label correctly describes the column's data format
3. Cross-reference with `eval/schema_mapping.yaml` to verify the short label resolves to the correct full FineType type
4. Flag any label that is wrong, misleading, or could be more precise
5. For ambiguous types, consider column header context per the mapping's partial-match rules

Judgement criteria:
- **WRONG:** Label is factually incorrect for the observed values
- **IMPRECISE:** Label is technically acceptable via partial-match rules but a more specific label exists and would improve eval signal
- **MAPPING ISSUE:** The short label is fine but its schema_mapping.yaml resolution is wrong
- **OK:** Label and mapping are correct

## Summary

```
Total columns reviewed:  338
Correct (no change):     318
Corrections needed:       20
  - Wrong labels:           7
  - Imprecise labels:      10
  - Mapping issues:         3
```

## Corrections

### Wrong Labels

These gt_labels are factually wrong for the observed column values.

```
| # | Dataset          | Column       | Old gt_label | New gt_label           | Sample Values                                    | Reasoning                                                        |
|---|------------------|--------------|--------------|------------------------|--------------------------------------------------|------------------------------------------------------------------|
| 1 | people_directory | ssn          | code         | ssn                    | 158-86-7620, 830-38-9759, 881-33-6499            | Values are US SSNs (NNN-NN-NNNN). "code" is too generic; these   |
|   |                  |              |              |                        |                                                  | are specifically SSNs. The ssn type exists in the taxonomy.       |
| 2 | codes_and_ids    | iban         | code         | iban                   | GB4210300104433879072333, DE1012152334132357259895| Values are clearly IBAN format (country prefix + digits). An      |
|   |                  |              |              |                        |                                                  | "iban" mapping does not exist in schema_mapping but "code" maps  |
|   |                  |              |              |                        |                                                  | to alphanumeric_id which is wrong. Needs new mapping entry.      |
| 3 | codes_and_ids    | locale       | language code| locale code            | de-DE, ko-KR, ar-SA, es-ES, hi-IN                | Values are BCP 47 locale codes (lang-REGION), not bare language  |
|   |                  |              |              |                        |                                                  | codes (en, fr). "language code" maps to technology.code.locale   |
|   |                  |              |              |                        |                                                  | which happens to be correct, but the label should say "locale    |
|   |                  |              |              |                        |                                                  | code" to match the actual type name locale_code.                 |
| 4 | network_logs     | method       | category     | http method            | GET, POST, PUT, DELETE, PATCH                     | These are HTTP methods, not generic categories. FineType has     |
|   |                  |              |              |                        |                                                  | technology.internet.http_method. The server_logs_json dataset    |
|   |                  |              |              |                        |                                                  | correctly labels its method column as "http method".             |
| 5 | medical_records  | diagnosis    | code         | icd10                  | B53.3, I14.7, F11.1, G57.4, D24.3                | Values are ICD-10 diagnosis codes (letter + digits + dot +       |
|   |                  |              |              |                        | _code                                            | digits). identity.medical.icd10 exists in taxonomy. "code" maps |
|   |                  |              |              |                        |                                                  | to alphanumeric_id which loses the specific type.                |
| 6 | network_logs     | timestamp    | timestamp    | iso timestamp          | 2024-12-30T14:34:10.000Z                         | All values have .000Z milliseconds (always zero). The format is  |
|   |                  |              |              | milliseconds           |                                                  | iso_8601_milliseconds, not generic "timestamp". Using the more   |
|   |                  |              |              |                        |                                                  | specific label matches the actual format and gives better eval   |
|   |                  |              |              |                        |                                                  | signal. (The generic "timestamp" is a partial match so this      |
|   |                  |              |              |                        |                                                  | wouldn't count as wrong in eval, but it's imprecise.)            |
| 7 | people_directory | salary       | price        | number                 | 229080, 172989, 133135, 218440, 122159            | Values are plain integers with no currency symbol, no decimal.   |
|   |                  |              |              |                        |                                                  | "price" maps to finance.currency.amount, but these are just      |
|   |                  |              |              |                        |                                                  | integer numbers. They are salaries semantically but FORMAT-wise  |
|   |                  |              |              |                        |                                                  | they are integer_number / number.                                |
```

### Imprecise Labels

These are technically acceptable via partial-match rules in schema_mapping.yaml but a more specific label would improve eval quality.

```
| # | Dataset              | Column       | Current gt_label  | Better gt_label             | Sample Values                                    | Reasoning                                                     |
|---|----------------------|--------------|-------------------|-----------------------------|--------------------------------------------------|---------------------------------------------------------------|
| 1 | datetime_formats     | us_date      | date              | mdy slash                   | 07/26/2022, 11/18/2021, 07/28/2021               | Unambiguously MM/DD/YYYY format. "date" is partial-match      |
|   |                      |              |                   |                             |                                                  | accepting any date type. Using "mdy slash" would test the     |
|   |                      |              |                   |                             |                                                  | engine's ability to distinguish US vs EU date formats.        |
| 2 | datetime_formats     | eu_date      | date              | dmy slash                   | 26/07/2022, 18/11/2021, 28/07/2021               | Unambiguously DD/MM/YYYY format. Same reasoning as us_date.   |
| 3 | datetime_formats     | iso_date     | date              | iso date                    | 2022-07-26, 2021-11-18, 2021-07-28               | Unambiguously YYYY-MM-DD ISO format. More precise label.      |
| 4 | covid_timeseries     | Date         | date              | iso date                    | 2020-01-22, 2020-01-23                           | All values are YYYY-MM-DD ISO format.                         |
| 5 | ecommerce_orders     | order_date   | date              | iso date                    | 2023-10-09, 2023-01-28, 2024-01-23               | All values are YYYY-MM-DD ISO format.                         |
| 6 | financial_data       | date         | date              | iso date                    | 2023-12-17, 2023-06-29, 2023-02-27               | All values are YYYY-MM-DD ISO format.                         |
| 7 | people_directory     | date_of_birth| date              | iso date                    | 2009-04-05, 2007-04-08, 1992-05-13               | All values are YYYY-MM-DD ISO format.                         |
| 8 | medical_records      | date_of_birth| date              | iso date                    | 2015-04-22, 1959-02-11, 1969-06-22               | All values are YYYY-MM-DD ISO format.                         |
| 9 | medical_records      | visit_date   | date              | iso date                    | 2023-10-04, 2023-11-13, 2023-08-09               | All values are YYYY-MM-DD ISO format.                         |
|10 | sports_events        | event_date   | date              | iso date                    | 2021-04-05, 2024-04-23, 2022-02-03               | All values are YYYY-MM-DD ISO format.                         |
```

**Note:** The spec context mentions that `iso_8601` catch-all was previously masking misclassifications where us_date and eu_date were labeled as iso_8601. The current manifest labels them as generic "date" which is the partial-match approach. Making them more specific (mdy_slash, dmy_slash, iso_date) would give much better eval signal for detecting date format confusion.

Additional imprecise labels considered but NOT flagged (acceptable as-is):

- `tech_systems/log_timestamp` (timestamp, iso_8601) — "timestamp" partial match is fine
- `scientific_measurements/timestamp` (timestamp, iso_8601) — same

### Mapping Issues

These are cases where the schema_mapping.yaml itself has problems or needs new entries.

```
| # | gt_label       | Current mapping                            | Issue                                                          | Suggested fix                              |
|---|----------------|--------------------------------------------|----------------------------------------------------------------|--------------------------------------------|
| 1 | iban           | No dedicated entry; falls to "code" →      | IBAN is a well-defined format (2-letter country + 2 check      | Add: iban → finance.banking.iban           |
|   |                | alphanumeric_id                            | digits + up to 30 alphanumeric). The taxonomy has              |                                            |
|   |                |                                            | finance.banking.iban. Currently codes_and_ids labels it "code".|                                            |
| 2 | mdy slash      | No mapping entry                           | If datetime_formats/us_date is changed from "date" to          | Add: mdy slash → datetime.date.mdy_slash   |
|   |                |                                            | "mdy slash", needs a new mapping entry. Pattern follows        |                                            |
|   |                |                                            | existing "mdy dash" and "dmy dash" entries.                    |                                            |
| 3 | dmy slash      | No mapping entry                           | Same as above for eu_date. Needs a new mapping entry.          | Add: dmy slash → datetime.date.dmy_slash   |
```

**Existing mappings already present (no schema_mapping changes needed):**
- `ssn` → identity.government.ssn (line 2314)
- `icd10` → identity.medical.icd10 (line 2278) — just change manifest label from "code" to "icd10"
- `http method` → technology.internet.http_method (line 2105) — just change manifest label from "category" to "http method"
- `iso date` → datetime.date.iso (line 2086) — just change manifest labels from "date" to "iso date"
- `iso timestamp milliseconds` → datetime.timestamp.iso_8601_milliseconds (line 2078) — just change manifest label from "timestamp" to "iso timestamp milliseconds"

## Columns Verified Correct (Notable Cases)

These columns warranted extra scrutiny but are correctly labeled:

```
| Dataset              | Column              | gt_label              | Notes                                                    |
|----------------------|---------------------|-----------------------|----------------------------------------------------------|
| titanic              | Survived            | boolean               | Values: 0, 1. Correct (binary boolean).                  |
| titanic              | Embarked            | category              | Values: S, C, Q. Correct (categorical).                  |
| titanic              | Fare                | price                 | Values: 7.25, 71.2833. These are monetary amounts.       |
|                      |                     |                       | Acceptable as "price" even without currency symbol.       |
| ecommerce_orders     | currency            | currency               | Values: EUR, CAD, GBP, USD. Maps to currency_code. OK.  |
| ecommerce_orders     | order_id            | code                  | Values: ORD-93810. Could be "alphanumeric id" but "code" |
|                      |                     |                       | maps to alphanumeric_id among others. Acceptable.        |
| financial_data       | market_cap          | value                 | Values: 1099.1B, 679.1B. SI number format. "value" maps  |
|                      |                     |                       | to si_number among others. Acceptable.                   |
| financial_data       | ticker              | code                  | Values: AAPL, MSFT, JNJ. Stock tickers. No ticker_symbol |
|                      |                     |                       | type in FineType taxonomy, so "code" is acceptable.      |
| airports             | utc_offset          | utc offset            | Values: 10, -3, -3.5. Numeric offsets, not +HH:MM        |
|                      |                     |                       | format. "utc offset" maps to datetime.offset.utc.        |
|                      |                     |                       | Borderline — these are numeric hours, not formatted UTC   |
|                      |                     |                       | offsets. But the mapping accepts it. Keep as-is.          |
| network_logs         | url_path            | code                  | Values: /health, /api/users. Could be "route" but "code" |
|                      |                     |                       | is acceptable via partial match.                          |
| network_logs         | query_params        | code                  | Values: page=23&limit=20&sort=asc. Query string format.   |
|                      |                     |                       | "code" maps to query_string among others. Acceptable.    |
| medical_records      | patient_id          | id                    | Values: PT-276718. Alphanumeric ID. "id" maps to          |
|                      |                     |                       | alphanumeric_id among others. Correct.                   |
| scientific_measurements | experiment_id    | id                    | Values: EXP-7620. Same pattern. Correct.                 |
| scientific_measurements | formula          | code                  | Values: NaCl, CH4, CO2. Chemical formulas. No SMILES-     |
|                      |                     |                       | style type match. "code" is acceptable as generic.        |
| codes_and_ids        | semantic_version    | version               | Values: 0.2.53, 6.27.84. Semver format. Correct.        |
| tech_systems         | language            | language              | Values: Rust, Python, C++. Programming language names.    |
|                      |                     |                       | "language" maps to locale_code or categorical. These      |
|                      |                     |                       | are categorical in practice. Acceptable.                  |
| books_catalog        | language            | language              | Values: English, French, Spanish. Natural language names.  |
|                      |                     |                       | Same mapping. Acceptable (categorical).                   |
| multilingual         | date                | date                  | Mixed formats: dd.mm.yyyy (de), yyyy/mm/dd (ja),         |
|                      |                     |                       | dd/mm/yyyy (pt-BR). "date" partial match is the only      |
|                      |                     |                       | sensible label for a mixed-format column.                 |
| ecommerce_orders_json| total               | decimal number        | Values: 99.98, 34.5, 129.0. Decimal numbers. Correct.    |
| ecommerce_orders_json| product             | category              | Values: Wireless Headphones, USB-C Hub. Product names     |
|                      |                     |                       | from a small set. Categorical is acceptable.              |
| earthquakes_2024     | place               | address               | Values: "80 km NW of Kandrian, Papua New Guinea". These   |
|                      |                     |                       | are location descriptions, not structured addresses.      |
|                      |                     |                       | "address" maps to geography.address.full_address which    |
|                      |                     |                       | is a stretch but acceptable via partial match.            |
| us_states            | Abbreviation        | code                  | Values: AL, AK, AZ. US state abbreviations. Could be      |
|                      |                     |                       | "state code" but no such type/mapping exists. "code"      |
|                      |                     |                       | mapping to generic identifiers is acceptable.             |
| medical_records      | is_admitted         | boolean               | Values: yes, No, YES, no. Boolean terms. Correct.        |
```

## Datasets with ALL Labels Correct

The following 26 datasets had zero corrections needed (232 columns total):

- titanic (12 columns)
- airports (13 columns)
- countries (9 columns)
- us_states (2 columns)
- world_cities (4 columns)
- iris (5 columns)
- tech_systems (12 columns)
- datetime_formats_extended (8 columns)
- geography_data (11 columns)
- books_catalog (11 columns)
- scientific_measurements (11 columns)
- multilingual (8 columns)
- api_users_json (7 columns)
- ecommerce_orders_json (10 columns)
- server_logs_json (9 columns)
- weather_stations_json (10 columns)
- new_geography (10 columns)
- new_technology (11 columns)
- new_identity (15 columns)
- new_finance (3 columns)
- new_representation (4 columns)
- earthquakes_2024 (22 columns)
- datetime_coverage (12 columns)
- finance_coverage (5 columns)
- representation_coverage (4 columns)
- technology_coverage (4 columns)

**Datasets with corrections (9 datasets, 106 columns, 20 corrections):**
- people_directory (14 columns, 3 corrections: ssn, salary, date_of_birth)
- codes_and_ids (12 columns, 2 corrections: iban, locale)
- network_logs (12 columns, 2 corrections: method, timestamp)
- medical_records (13 columns, 3 corrections: diagnosis_code, date_of_birth, visit_date)
- datetime_formats (14 columns, 3 corrections: us_date, eu_date, iso_date)
- covid_timeseries (5 columns, 1 correction: Date)
- ecommerce_orders (12 columns, 1 correction: order_date)
- financial_data (12 columns, 1 correction: date)
- sports_events (12 columns, 1 correction: event_date)

## Recommended Priority

**High (fix before v16 retrain):**
1. people_directory/ssn: code → ssn (wrong type entirely)
2. codes_and_ids/iban: code → iban (wrong type, needs mapping entry)
3. medical_records/diagnosis_code: code → icd10 (wrong type)
4. network_logs/method: category → http method (wrong type)
5. people_directory/salary: price → number (wrong base type)

**Medium (improves eval signal):**
6. datetime_formats/us_date: date → mdy slash (needs new mapping)
7. datetime_formats/eu_date: date → dmy slash (needs new mapping)
8. datetime_formats/iso_date: date → iso date
9. covid_timeseries/Date: date → iso date
10. ecommerce_orders/order_date: date → iso date
11. financial_data/date: date → iso date
12. people_directory/date_of_birth: date → iso date
13. medical_records/date_of_birth: date → iso date
14. medical_records/visit_date: date → iso date
15. sports_events/event_date: date → iso date
16. codes_and_ids/locale: language code → locale code (cosmetic but correct)
17. network_logs/timestamp: timestamp → iso timestamp milliseconds

**Low (cosmetic, acceptable as-is):**
- No remaining items — all imprecise labels moved to medium priority

## Impact Assessment

If all high-priority corrections are applied, the eval baseline will shift:
- 5 columns will get more specific ground truth labels
- Any previously "correct" predictions that matched via partial/generic fallback will now require exact type match
- This is healthy: it exposes real model weaknesses instead of masking them behind generic labels
- Expected short-term accuracy drop: ~2-3 columns (model may not distinguish SSN from code, IBAN from alphanumeric_id, or ICD-10 from code without retraining)
- Expected long-term benefit: cleaner training signal for v16
