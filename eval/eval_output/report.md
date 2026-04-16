# FineType Evaluation Report

**Generated:** 2026-04-16 19:40

## Headline Metrics

| Metric | Value | Status |
|---|---|---|
| Profile label accuracy | 204/227 (89.9%) | 🟡 |
| Profile domain accuracy | 210/227 (92.5%) | 🟡 |
| Actionability (datetime) | 544198/545091 (99.8%) | 🟢 |
| Columns with >95% parse rate | 310/328 | |
| Taxonomy types | 240 | |
| Types with format_string | 65 | |
| Types with validation | 240 | |
| Types with locale validation | 6 | |

## Taxonomy Coverage

| Domain | Types |
|---|---|
| container | 11 |
| datetime | 84 |
| finance | 28 |
| geography | 25 |
| identity | 33 |
| representation | 33 |
| technology | 26 |

## Profile Evaluation

**Label accuracy:** 204/227 (89.9%)
**Domain accuracy:** 210/227 (92.5%)

### Misclassifications

| Dataset | Column | Predicted | Expected | Confidence |
|---|---|---|---|---|
| new_technology | data_uri | url | data_uri | 1.00 |
| new_identity | email_display | email | email_display | 1.00 |
| new_identity | phone_e164 | phone_number | phone_e164 | 1.00 |
| weather_stations_json | location.country | country | country_code | 1.00 |
| api_users_json | address.country | country | country_code | 1.00 |
| earthquakes_2024 | depthError | latitude | decimal_number | 1.00 |
| people_directory | phone | ssn | phone_number | 1.00 |
| tech_systems | user_agent | jwt | user_agent | 1.00 |
| codes_and_ids | sha256 | ethereum_address | hash | 0.99 |
| network_logs | status_code | postal_code | integer_number | 0.99 |
| earthquakes_2024 | id | geohash | alphanumeric_id | 0.99 |
| datetime_formats | year | compact_ym | year | 0.98 |
| ecommerce_orders | phone | ssn | phone_number | 0.98 |
| earthquakes_2024 | dmin | latitude | decimal_number | 0.97 |
| datetime_formats_extended | long_full_month_date | dmy_space_full | long_full_month | 0.94 |
| server_logs_json | status_code | postal_code | integer_number | 0.80 |
| new_technology | git_sha | tsid | hash | 0.74 |
| server_logs_json | method | iata_code | http_method | 0.63 |
| new_geography | geojson | plain_text | json | 0.61 |
| scientific_measurements | measurement_unit | country_code | measurement_unit | 0.49 |
| network_logs | user_agent | url | user_agent | 0.48 |
| earthquakes_2024 | gap | amount_accounting | decimal_number | 0.43 |
| multilingual | locale | alphanumeric_id | locale_code | 0.36 |

## Precision Per Type (Profile Eval)

| Predicted Type | Predicted | Correct | Precision | Status |
|---|---|---|---|---|
| decimal_number | 20 | 20 | 100% | 🟢 |
| country | 10 | 8 | 80% | 🟡 |
| latitude | 7 | 5 | 71.4% | 🔴 |
| url | 7 | 5 | 71.4% | 🔴 |
| iso_8601 | 6 | 6 | 100% | 🟢 |
| postal_code | 6 | 4 | 66.7% | 🔴 |
| ip_v4 | 5 | 5 | 100% | 🟢 |
| email | 5 | 4 | 80% | 🟡 |
| longitude | 5 | 5 | 100% | 🟢 |
| state | 5 | 5 | 100% | 🟢 |
| full_name | 5 | 5 | 100% | 🟢 |
| city | 5 | 5 | 100% | 🟢 |
| entity_name | 4 | 4 | 100% | 🟢 |
| full_address | 4 | 4 | 100% | 🟢 |
| country_code | 4 | 3 | 75% | 🔴 |
| terms | 4 | 4 | 100% | 🟢 |
| percentage | 3 | 3 | 100% | 🟢 |
| uuid | 3 | 3 | 100% | 🟢 |
| phone_number | 3 | 2 | 66.7% | 🔴 |
| gender | 3 | 3 | 100% | 🟢 |
| ssn | 3 | 1 | 33.3% | 🔴 |
| integer_number | 3 | 3 | 100% | 🟢 |
| first_name | 2 | 2 | 100% | 🟢 |
| dmy_dash | 2 | 2 | 100% | 🟢 |
| height | 2 | 2 | 100% | 🟢 |
| last_name | 2 | 2 | 100% | 🟢 |
| alphanumeric_id | 2 | 1 | 50% | 🔴 |
| iata_code | 2 | 1 | 50% | 🔴 |
| geohash | 2 | 1 | 50% | 🔴 |
| utc | 2 | 2 | 100% | 🟢 |
| iana | 2 | 2 | 100% | 🟢 |
| currency_code | 2 | 2 | 100% | 🟢 |
| jwt | 2 | 1 | 50% | 🔴 |
| weight | 2 | 2 | 100% | 🟢 |
| tsid | 2 | 1 | 50% | 🔴 |
| ean | 2 | 2 | 100% | 🟢 |
| figi | 1 | 1 | 100% | 🟢 |
| smiles | 1 | 1 | 100% | 🟢 |
| calver | 1 | 1 | 100% | 🟢 |
| h3 | 1 | 1 | 100% | 🟢 |
| unlocode | 1 | 1 | 100% | 🟢 |
| compact_dmy | 1 | 1 | 100% | 🟢 |
| docker_ref | 1 | 1 | 100% | 🟢 |
| plus_code | 1 | 1 | 100% | 🟢 |
| ein | 1 | 1 | 100% | 🟢 |
| iso6346 | 1 | 1 | 100% | 🟢 |
| iso_8601_milliseconds | 1 | 1 | 100% | 🟢 |
| lei | 1 | 1 | 100% | 🟢 |
| bitcoin_address | 1 | 1 | 100% | 🟢 |
| orcid | 1 | 1 | 100% | 🟢 |
| swift_bic | 1 | 1 | 100% | 🟢 |
| cas_number | 1 | 1 | 100% | 🟢 |
| scientific_notation | 1 | 1 | 100% | 🟢 |
| decimal_number_comma | 1 | 1 | 100% | 🟢 |
| ymd_dot | 1 | 1 | 100% | 🟢 |
| vin | 1 | 1 | 100% | 🟢 |
| dmy_space_full | 1 | 0 | 0% | 🔴 |
| urn | 1 | 1 | 100% | 🟢 |
| isrc | 1 | 1 | 100% | 🟢 |
| s3_uri | 1 | 1 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 1 | 100% | 🟢 |
| inchi | 1 | 1 | 100% | 🟢 |
| categorical | 1 | 1 | 100% | 🟢 |
| wkt | 1 | 1 | 100% | 🟢 |
| issn | 1 | 1 | 100% | 🟢 |
| month_name | 1 | 1 | 100% | 🟢 |
| mgrs | 1 | 1 | 100% | 🟢 |
| isin | 1 | 1 | 100% | 🟢 |
| iso | 1 | 1 | 100% | 🟢 |
| hms_12h | 1 | 1 | 100% | 🟢 |
| plain_text | 1 | 0 | 0% | 🔴 |
| rfc_3339 | 1 | 1 | 100% | 🟢 |
| hostname | 1 | 1 | 100% | 🟢 |
| amount_accounting | 1 | 0 | 0% | 🔴 |
| rfc_2822 | 1 | 1 | 100% | 🟢 |
| hs_code | 1 | 1 | 100% | 🟢 |
| abn | 1 | 1 | 100% | 🟢 |
| compact_ymd | 1 | 1 | 100% | 🟢 |
| cusip | 1 | 1 | 100% | 🟢 |
| cidr | 1 | 1 | 100% | 🟢 |
| compact_ym | 1 | 0 | 0% | 🔴 |
| snowflake_id | 1 | 1 | 100% | 🟢 |
| bsb | 1 | 1 | 100% | 🟢 |
| dmy_dot | 1 | 1 | 100% | 🟢 |
| quarter | 1 | 1 | 100% | 🟢 |
| syslog_bsd | 1 | 1 | 100% | 🟢 |
| iso_week | 1 | 1 | 100% | 🟢 |
| aba_routing | 1 | 1 | 100% | 🟢 |
| credit_card_number | 1 | 1 | 100% | 🟢 |
| day_of_week | 1 | 1 | 100% | 🟢 |
| iso_8601_offset | 1 | 1 | 100% | 🟢 |
| binary | 1 | 1 | 100% | 🟢 |
| npi | 1 | 1 | 100% | 🟢 |
| cpt | 1 | 1 | 100% | 🟢 |
| locale_code | 1 | 1 | 100% | 🟢 |
| ethereum_address | 1 | 0 | 0% | 🔴 |
| year | 1 | 1 | 100% | 🟢 |
| color_hsl | 1 | 1 | 100% | 🟢 |
| mac_address | 1 | 1 | 100% | 🟢 |
| sql_standard | 1 | 1 | 100% | 🟢 |
| hm_12h | 1 | 1 | 100% | 🟢 |
| pan_india | 1 | 1 | 100% | 🟢 |
| hcpcs | 1 | 1 | 100% | 🟢 |
| ulid | 1 | 1 | 100% | 🟢 |
| user_agent | 1 | 1 | 100% | 🟢 |
| icd10 | 1 | 1 | 100% | 🟢 |
| icao_code | 1 | 1 | 100% | 🟢 |
| fiscal_year | 1 | 1 | 100% | 🟢 |
| dms | 1 | 1 | 100% | 🟢 |
| ip_v6 | 1 | 1 | 100% | 🟢 |
| loinc | 1 | 1 | 100% | 🟢 |
| username | 1 | 1 | 100% | 🟢 |
| eu_vat | 1 | 1 | 100% | 🟢 |
| aws_arn | 1 | 1 | 100% | 🟢 |
| file_size | 1 | 1 | 100% | 🟢 |

## Actionability Evaluation

Can analysts safely TRY_CAST using FineType's format_string predictions?
**Target:** >95% success rate for datetime types

### By Type

| Type | Columns | Values | Success Rate | Status |
|---|---|---|---|---|
| integer_number | 23 | 50164 | 100% | 🟢 |
| decimal_number | 21 | 71763 | 99.9% | 🟢 |
| iso_8601 | 13 | 14897 | 97.1% | 🟢 |
| amount | 11 | 1736 | 100% | 🟢 |
| country | 10 | 42664 | 100% | 🟢 |
| categorical | 9 | 1604 | 100% | 🟢 |
| url | 8 | 605 | 100% | 🟢 |
| iso | 7 | 1500 | 100% | 🟢 |
| postal_code | 7 | 545 | 100% | 🟢 |
| latitude | 7 | 50252 | 100% | 🟢 |
| city | 6 | 55238 | 100% | 🟢 |
| entity_name | 6 | 8043 | 100% | 🟢 |
| longitude | 5 | 22040 | 100% | 🟢 |
| country_code | 5 | 897 | 100% | 🟢 |
| email | 5 | 365 | 100% | 🟢 |
| full_name | 5 | 1171 | 100% | 🟢 |
| alphanumeric_id | 5 | 335 | 100% | 🟢 |
| ip_v4 | 5 | 330 | 100% | 🟢 |
| currency_code | 4 | 250 | 100% | 🟢 |
| full_address | 4 | 21990 | 100% | 🟢 |
| ssn | 4 | 380 | 100% | 🟢 |
| terms | 4 | 285 | 100% | 🟢 |
| ean | 3 | 260 | 100% | 🟢 |
| gender | 3 | 1051 | 100% | 🟢 |
| phone_number | 3 | 200 | 100% | 🟢 |
| ordinal | 3 | 976 | 100% | 🟢 |
| uuid | 3 | 260 | 100% | 🟢 |
| percentage | 3 | 250 | 100% | 🟢 |
| measurement_unit | 3 | 42396 | 100% | 🟢 |
| top_level_domain | 3 | 35962 | 100% | 🟢 |
| whitespace_separated | 2 | 120 | 100% | 🟢 |
| dmy_dash | 2 | 50 | 68% | 🔴 |
| dmy_short_dot | 2 | 160 | 61.3% | 🔴 |
| iso_week | 2 | 85 | 29.4% | 🔴 |
| iana | 2 | 7778 | 100% | 🟢 |
| utc | 2 | 7778 | 100% | 🟢 |
| credit_card_number | 2 | 160 | 100% | 🟢 |
| geohash | 2 | 14212 | 100% | 🟢 |
| iata_code | 2 | 7723 | 100% | 🟢 |
| icao_code | 2 | 7798 | 100% | 🟢 |
| isbn | 2 | 140 | 100% | 🟢 |
| eu_vat | 2 | 329 | 100% | 🟢 |
| icd10 | 2 | 140 | 100% | 🟢 |
| first_name | 2 | 160 | 100% | 🟢 |
| height | 2 | 160 | 100% | 🟢 |
| last_name | 2 | 160 | 100% | 🟢 |
| weight | 2 | 160 | 100% | 🟢 |
| file_size | 2 | 125 | 100% | 🟢 |
| mime_type | 2 | 180 | 100% | 🟢 |
| increment | 2 | 34128 | 100% | 🟢 |
| smiles | 2 | 130 | 100% | 🟢 |
| plain_text | 2 | 105 | 100% | 🟢 |
| jwt | 2 | 160 | 100% | 🟢 |
| tsid | 2 | 160 | 100% | 🟢 |
| query_string | 1 | 100 | 100% | 🟢 |
| day_of_week | 1 | 80 | 100% | 🟢 |
| month_name | 1 | 80 | 100% | 🟢 |
| periodicity | 1 | 100 | 100% | 🟢 |
| year | 1 | 60 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 80 | 0% | 🔴 |
| compact_dmy | 1 | 25 | 100% | 🟢 |
| compact_ym | 1 | 80 | 0% | 🔴 |
| compact_ymd | 1 | 25 | 100% | 🟢 |
| dmy_dot | 1 | 80 | 100% | 🟢 |
| dmy_space_full | 1 | 80 | 0% | 🔴 |
| ymd_dot | 1 | 25 | 100% | 🟢 |
| iso_8601 | 1 | 80 | 100% | 🟢 |
| unix_milliseconds | 1 | 80 | 100% | 🟢 |
| unix_seconds | 1 | 80 | 100% | 🟢 |
| fiscal_year | 1 | 25 | 100% | 🟢 |
| quarter | 1 | 25 | 0% | 🔴 |
| hm_12h | 1 | 80 | 100% | 🟢 |
| hm_24h | 1 | 100 | 100% | 🟢 |
| hms_12h | 1 | 80 | 100% | 🟢 |
| hms_24h | 1 | 80 | 100% | 🟢 |
| iso_8601_milliseconds | 1 | 14132 | 100% | 🟢 |
| iso_8601_offset | 1 | 25 | 100% | 🟢 |
| rfc_2822 | 1 | 80 | 100% | 🟢 |
| rfc_3339 | 1 | 0 | 0% | 🔴 |
| sql_standard | 1 | 80 | 100% | 🟢 |
| syslog_bsd | 1 | 0 | 0% | 🔴 |
| aba_routing | 1 | 80 | 100% | 🟢 |
| bsb | 1 | 80 | 100% | 🟢 |
| swift_bic | 1 | 80 | 100% | 🟢 |
| bitcoin_address | 1 | 25 | 100% | 🟢 |
| ethereum_address | 1 | 80 | 100% | 🟢 |
| amount_accounting | 1 | 14080 | 100% | 🟢 |
| cusip | 1 | 25 | 100% | 🟢 |
| figi | 1 | 80 | 100% | 🟢 |
| isin | 1 | 25 | 100% | 🟢 |
| lei | 1 | 25 | 100% | 🟢 |
| coordinates | 1 | 100 | 100% | 🟢 |
| dms | 1 | 80 | 100% | 🟢 |
| mgrs | 1 | 80 | 100% | 🟢 |
| plus_code | 1 | 80 | 100% | 🟢 |
| wkt | 1 | 80 | 100% | 🟢 |
| h3 | 1 | 80 | 100% | 🟢 |
| region | 1 | 51 | 100% | 🟢 |
| hs_code | 1 | 80 | 100% | 🟢 |
| iso6346 | 1 | 80 | 100% | 🟢 |
| unlocode | 1 | 80 | 100% | 🟢 |
| orcid | 1 | 80 | 100% | 🟢 |
| isrc | 1 | 80 | 100% | 🟢 |
| issn | 1 | 80 | 100% | 🟢 |
| abn | 1 | 80 | 100% | 🟢 |
| ein | 1 | 80 | 100% | 🟢 |
| pan_india | 1 | 80 | 100% | 🟢 |
| vin | 1 | 80 | 100% | 🟢 |
| cpt | 1 | 80 | 100% | 🟢 |
| hcpcs | 1 | 80 | 100% | 🟢 |
| loinc | 1 | 80 | 100% | 🟢 |
| npi | 1 | 60 | 100% | 🟢 |
| username | 1 | 25 | 100% | 🟢 |
| binary | 1 | 891 | 100% | 🟢 |
| color_hex | 1 | 80 | 100% | 🟢 |
| color_hsl | 1 | 80 | 100% | 🟢 |
| numeric_code | 1 | 247 | 100% | 🟢 |
| decimal_number_comma | 1 | 25 | 100% | 🟢 |
| scientific_notation | 1 | 25 | 100% | 🟢 |
| cas_number | 1 | 80 | 100% | 🟢 |
| inchi | 1 | 80 | 100% | 🟢 |
| aws_arn | 1 | 80 | 100% | 🟢 |
| s3_uri | 1 | 80 | 100% | 🟢 |
| locale_code | 1 | 80 | 100% | 🟢 |
| token_urlsafe | 1 | 80 | 100% | 🟢 |
| calver | 1 | 25 | 100% | 🟢 |
| docker_ref | 1 | 80 | 100% | 🟢 |
| snowflake_id | 1 | 80 | 100% | 🟢 |
| ulid | 1 | 80 | 100% | 🟢 |
| cidr | 1 | 80 | 100% | 🟢 |
| hostname | 1 | 80 | 100% | 🟢 |
| http_method | 1 | 100 | 100% | 🟢 |
| ip_v6 | 1 | 25 | 100% | 🟢 |
| mac_address | 1 | 80 | 100% | 🟢 |
| urn | 1 | 80 | 100% | 🟢 |
| user_agent | 1 | 25 | 100% | 🟢 |

### Below Target (<95%)

| Dataset | Column | Type | Format | Success Rate |
|---|---|---|---|---|
| datetime_formats | us_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats | eu_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats | year | compact_ym | `` | 0% 🔴 |
| medical_records | patient_id | iso_week | `` | 0% 🔴 |
| multilingual | date | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | abbreviated_month_date | abbrev_month_no_comma | `` | 0% 🔴 |
| datetime_formats_extended | long_full_month_date | dmy_space_full | `` | 0% 🔴 |
| datetime_formats_extended | american_timestamp | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | european_timestamp | iso_8601 | `` | 0% 🔴 |
| ecommerce_orders_json | order_date | iso_8601 | `` | 0% 🔴 |
| datetime_coverage | rfc_3339 | rfc_3339 | `` | 0% 🔴 |
| datetime_coverage | clf_timestamp | iso_8601 | `` | 0% 🔴 |
| datetime_coverage | syslog_bsd | syslog_bsd | `` | 0% 🔴 |
| medical_records | blood_pressure | decimal_number | `` | 0% 🔴 |
| datetime_coverage | quarter | quarter | `` | 0% 🔴 |
| codes_and_ids | semantic_version | dmy_short_dot | `` | 28.7% 🔴 |
| datetime_coverage | mdy_dash | dmy_dash | `` | 36% 🔴 |
| tech_systems | version | dmy_short_dot | `` | 93.8% 🔴 |

## Evaluation Components

| Component | Scope | Target | Status |
|---|---|---|---|
| Profile regression | 227 columns, 35 datasets | No regressions | 🟡 |
| Precision per type | SOTAB/GitTables | 🟢≥95% per type | Run `make eval-sotab-cli` |
| Overcall analysis | SOTAB/GitTables | <5% FP rate | Run `make eval-sotab-cli` |
| Actionability | Profile eval datetime | >95% parse rate | 🟢 |
| Confidence calibration | SOTAB/GitTables | Gap <10pp | Run `make eval-sotab-cli` |
| Domain accuracy | SOTAB format-detectable | >80% | Run `make eval-sotab-cli` |

---
*Generated by eval-report (NNFT-184, Rust port of eval_report.py)*
