# FineType Evaluation Report

**Generated:** 2026-04-12 21:43

## Headline Metrics

| Metric | Value | Status |
|---|---|---|
| Profile label accuracy | 201/227 (88.5%) | 🟡 |
| Profile domain accuracy | 207/227 (91.2%) | 🟡 |
| Actionability (datetime) | 494505/511374 (96.7%) | 🟢 |
| Columns with >95% parse rate | 289/321 | |
| Taxonomy types | 239 | |
| Types with format_string | 65 | |
| Types with validation | 239 | |
| Types with locale validation | 5 | |

## Taxonomy Coverage

| Domain | Types |
|---|---|
| container | 11 |
| datetime | 84 |
| finance | 28 |
| geography | 24 |
| identity | 33 |
| representation | 33 |
| technology | 26 |

## Profile Evaluation

**Label accuracy:** 201/227 (88.5%)
**Domain accuracy:** 207/227 (91.2%)

### Misclassifications

| Dataset | Column | Predicted | Expected | Confidence |
|---|---|---|---|---|
| new_technology | data_uri | url | data_uri | 1.00 |
| new_identity | email_display | email | email_display | 1.00 |
| new_identity | phone_e164 | phone_number | phone_e164 | 1.00 |
| datetime_formats | year | compact_ym | year | 1.00 |
| api_users_json | address.country | country | country_code | 1.00 |
| weather_stations_json | location.country | country | country_code | 1.00 |
| tech_systems | user_agent | jwt | user_agent | 1.00 |
| earthquakes_2024 | depthError | latitude | decimal_number | 0.97 |
| network_logs | status_code | postal_code | integer_number | 0.89 |
| tech_systems | server_hostname | url | hostname | 0.86 |
| tech_systems | port | ean | integer_number | 0.82 |
| datetime_formats_extended | eu_dot_date | iso_8601 | dmy_dot | 0.78 |
| finance_coverage | isin | alphanumeric_id | isin | 0.75 |
| network_logs | user_agent | mdy_12h | user_agent | 0.70 |
| earthquakes_2024 | id | geohash | alphanumeric_id | 0.67 |
| iris | sepal_length | version | decimal_number | 0.66 |
| earthquakes_2024 | horizontalError | dmy_short_dot | decimal_number | 0.62 |
| financial_data | pe_ratio | latitude | decimal_number | 0.61 |
| datetime_coverage | mdy_dash | iso | mdy_dash | 0.59 |
| server_logs_json | status_code | postal_code | integer_number | 0.58 |
| iris | sepal_width | version | decimal_number | 0.57 |
| datetime_coverage | iso_week | iso | iso_week | 0.55 |
| server_logs_json | user_agent | plain_text | user_agent | 0.47 |
| representation_coverage | scientific_notation | plain_text | scientific_notation | 0.45 |
| datetime_coverage | dmy_dash | iso | dmy_dash | 0.39 |
| ecommerce_orders_json | order_id | isbn | alphanumeric_id | 0.34 |

## Precision Per Type (Profile Eval)

| Predicted Type | Predicted | Correct | Precision | Status |
|---|---|---|---|---|
| decimal_number | 17 | 17 | 100% | 🟢 |
| country | 10 | 8 | 80% | 🟡 |
| iso_8601 | 8 | 7 | 87.5% | 🟡 |
| latitude | 7 | 5 | 71.4% | 🔴 |
| url | 7 | 5 | 71.4% | 🔴 |
| postal_code | 6 | 4 | 66.7% | 🔴 |
| city | 5 | 5 | 100% | 🟢 |
| ip_v4 | 5 | 5 | 100% | 🟢 |
| state | 5 | 5 | 100% | 🟢 |
| phone_number | 5 | 4 | 80% | 🟡 |
| full_name | 5 | 5 | 100% | 🟢 |
| longitude | 5 | 5 | 100% | 🟢 |
| email | 5 | 4 | 80% | 🟡 |
| entity_name | 4 | 4 | 100% | 🟢 |
| terms | 4 | 4 | 100% | 🟢 |
| full_address | 3 | 3 | 100% | 🟢 |
| integer_number | 3 | 3 | 100% | 🟢 |
| uuid | 3 | 3 | 100% | 🟢 |
| ean | 3 | 2 | 66.7% | 🔴 |
| iso | 3 | 0 | 0% | 🔴 |
| country_code | 3 | 3 | 100% | 🟢 |
| percentage | 3 | 3 | 100% | 🟢 |
| categorical | 3 | 3 | 100% | 🟢 |
| gender | 3 | 3 | 100% | 🟢 |
| currency_code | 2 | 2 | 100% | 🟢 |
| first_name | 2 | 2 | 100% | 🟢 |
| iana | 2 | 2 | 100% | 🟢 |
| weight | 2 | 2 | 100% | 🟢 |
| jwt | 2 | 1 | 50% | 🔴 |
| height | 2 | 2 | 100% | 🟢 |
| last_name | 2 | 2 | 100% | 🟢 |
| version | 2 | 0 | 0% | 🔴 |
| utc | 2 | 2 | 100% | 🟢 |
| plain_text | 2 | 0 | 0% | 🔴 |
| locale_code | 2 | 2 | 100% | 🟢 |
| geohash | 2 | 1 | 50% | 🔴 |
| git_sha | 2 | 2 | 100% | 🟢 |
| iso6346 | 1 | 1 | 100% | 🟢 |
| mac_address | 1 | 1 | 100% | 🟢 |
| iata_code | 1 | 1 | 100% | 🟢 |
| compact_dmy | 1 | 1 | 100% | 🟢 |
| binary | 1 | 1 | 100% | 🟢 |
| inchi | 1 | 1 | 100% | 🟢 |
| issn | 1 | 1 | 100% | 🟢 |
| lei | 1 | 1 | 100% | 🟢 |
| compact_ym | 1 | 0 | 0% | 🔴 |
| npi | 1 | 1 | 100% | 🟢 |
| hms_12h | 1 | 1 | 100% | 🟢 |
| region | 1 | 1 | 100% | 🟢 |
| iso_8601_milliseconds | 1 | 1 | 100% | 🟢 |
| orcid | 1 | 1 | 100% | 🟢 |
| ymd_dot | 1 | 1 | 100% | 🟢 |
| ssn | 1 | 1 | 100% | 🟢 |
| clf | 1 | 1 | 100% | 🟢 |
| isrc | 1 | 1 | 100% | 🟢 |
| geojson | 1 | 1 | 100% | 🟢 |
| cidr | 1 | 1 | 100% | 🟢 |
| decimal_number_comma | 1 | 1 | 100% | 🟢 |
| month_name | 1 | 1 | 100% | 🟢 |
| dms | 1 | 1 | 100% | 🟢 |
| dmy_short_dot | 1 | 0 | 0% | 🔴 |
| rfc_2822 | 1 | 1 | 100% | 🟢 |
| icao_code | 1 | 1 | 100% | 🟢 |
| ip_v6 | 1 | 1 | 100% | 🟢 |
| aba_routing | 1 | 1 | 100% | 🟢 |
| iso_8601_offset | 1 | 1 | 100% | 🟢 |
| smiles | 1 | 1 | 100% | 🟢 |
| cpt | 1 | 1 | 100% | 🟢 |
| wkt | 1 | 1 | 100% | 🟢 |
| calver | 1 | 1 | 100% | 🟢 |
| isbn | 1 | 0 | 0% | 🔴 |
| compact_ymd | 1 | 1 | 100% | 🟢 |
| rfc_3339 | 1 | 1 | 100% | 🟢 |
| alphanumeric_id | 1 | 0 | 0% | 🔴 |
| abbrev_month_no_comma | 1 | 1 | 100% | 🟢 |
| year | 1 | 1 | 100% | 🟢 |
| full_month_no_comma | 1 | 1 | 100% | 🟢 |
| ein | 1 | 1 | 100% | 🟢 |
| vin | 1 | 1 | 100% | 🟢 |
| bsb | 1 | 1 | 100% | 🟢 |
| cas_number | 1 | 1 | 100% | 🟢 |
| ulid | 1 | 1 | 100% | 🟢 |
| hcpcs | 1 | 1 | 100% | 🟢 |
| swift_bic | 1 | 1 | 100% | 🟢 |
| color_hsl | 1 | 1 | 100% | 🟢 |
| fiscal_year | 1 | 1 | 100% | 🟢 |
| mgrs | 1 | 1 | 100% | 🟢 |
| loinc | 1 | 1 | 100% | 🟢 |
| username | 1 | 1 | 100% | 🟢 |
| figi | 1 | 1 | 100% | 🟢 |
| aws_arn | 1 | 1 | 100% | 🟢 |
| s3_uri | 1 | 1 | 100% | 🟢 |
| sql_standard | 1 | 1 | 100% | 🟢 |
| quarter | 1 | 1 | 100% | 🟢 |
| docker_ref | 1 | 1 | 100% | 🟢 |
| file_size | 1 | 1 | 100% | 🟢 |
| pan_india | 1 | 1 | 100% | 🟢 |
| bitcoin_address | 1 | 1 | 100% | 🟢 |
| unlocode | 1 | 1 | 100% | 🟢 |
| hm_12h | 1 | 1 | 100% | 🟢 |
| h3 | 1 | 1 | 100% | 🟢 |
| cusip | 1 | 1 | 100% | 🟢 |
| abn | 1 | 1 | 100% | 🟢 |
| credit_card_number | 1 | 1 | 100% | 🟢 |
| urn | 1 | 1 | 100% | 🟢 |
| hs_code | 1 | 1 | 100% | 🟢 |
| tsid | 1 | 1 | 100% | 🟢 |
| snowflake_id | 1 | 1 | 100% | 🟢 |
| eu_vat | 1 | 1 | 100% | 🟢 |
| mdy_12h | 1 | 0 | 0% | 🔴 |
| icd10 | 1 | 1 | 100% | 🟢 |
| day_of_week | 1 | 1 | 100% | 🟢 |
| plus_code | 1 | 1 | 100% | 🟢 |

## Actionability Evaluation

Can analysts safely TRY_CAST using FineType's format_string predictions?
**Target:** >95% success rate for datetime types

### By Type

| Type | Columns | Values | Success Rate | Status |
|---|---|---|---|---|
| integer_number | 22 | 63273 | 100% | 🟢 |
| iso_8601 | 21 | 16477 | 87.8% | 🟡 |
| decimal_number | 18 | 71371 | 99.9% | 🟢 |
| amount | 11 | 1736 | 100% | 🟢 |
| categorical | 11 | 15786 | 100% | 🟢 |
| country | 10 | 42664 | 100% | 🟢 |
| entity_name | 8 | 8128 | 100% | 🟢 |
| url | 8 | 585 | 100% | 🟢 |
| postal_code | 7 | 545 | 100% | 🟢 |
| latitude | 7 | 36272 | 100% | 🟢 |
| longitude | 6 | 22140 | 99.5% | 🟢 |
| city | 5 | 41106 | 100% | 🟢 |
| email | 5 | 365 | 100% | 🟢 |
| full_name | 5 | 1171 | 100% | 🟢 |
| phone_number | 5 | 400 | 100% | 🟢 |
| ip_v4 | 5 | 330 | 100% | 🟢 |
| iana | 4 | 29608 | 100% | 🟢 |
| currency_code | 4 | 250 | 100% | 🟢 |
| country_code | 4 | 847 | 100% | 🟢 |
| terms | 4 | 285 | 100% | 🟢 |
| ordinal | 4 | 1867 | 100% | 🟢 |
| dmy_short_dot | 3 | 14232 | 0.7% | 🔴 |
| iso | 3 | 75 | 0% | 🔴 |
| full_address | 3 | 7858 | 100% | 🟢 |
| continent | 3 | 42396 | 100% | 🟢 |
| ean | 3 | 240 | 100% | 🟢 |
| isbn | 3 | 165 | 100% | 🟢 |
| gender | 3 | 1051 | 100% | 🟢 |
| uuid | 3 | 260 | 100% | 🟢 |
| percentage | 3 | 250 | 100% | 🟢 |
| plain_text | 3 | 75 | 100% | 🟢 |
| utc | 2 | 7778 | 100% | 🟢 |
| geohash | 2 | 14212 | 100% | 🟢 |
| region | 2 | 14183 | 100% | 🟢 |
| icao_code | 2 | 7798 | 100% | 🟢 |
| isrc | 2 | 140 | 100% | 🟢 |
| ssn | 2 | 180 | 100% | 🟢 |
| icd10 | 2 | 140 | 100% | 🟢 |
| first_name | 2 | 160 | 100% | 🟢 |
| height | 2 | 160 | 100% | 🟢 |
| last_name | 2 | 160 | 100% | 🟢 |
| weight | 2 | 160 | 100% | 🟢 |
| mime_type | 2 | 180 | 100% | 🟢 |
| smiles | 2 | 130 | 100% | 🟢 |
| locale_code | 2 | 140 | 100% | 🟢 |
| jwt | 2 | 160 | 100% | 🟢 |
| version | 2 | 300 | 100% | 🟢 |
| snowflake_id | 2 | 80 | 100% | 🟢 |
| query_string | 1 | 100 | 100% | 🟢 |
| day_of_week | 1 | 80 | 100% | 🟢 |
| month_name | 1 | 80 | 100% | 🟢 |
| periodicity | 1 | 100 | 100% | 🟢 |
| year | 1 | 60 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 80 | 0% | 🔴 |
| compact_dmy | 1 | 25 | 100% | 🟢 |
| compact_ym | 1 | 80 | 0% | 🔴 |
| compact_ymd | 1 | 25 | 100% | 🟢 |
| full_month_no_comma | 1 | 80 | 0% | 🔴 |
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
| clf | 1 | 25 | 0% | 🔴 |
| iso_8601_milliseconds | 1 | 14132 | 100% | 🟢 |
| iso_8601_offset | 1 | 25 | 100% | 🟢 |
| mdy_12h | 1 | 100 | 0% | 🔴 |
| rfc_2822 | 1 | 80 | 100% | 🟢 |
| rfc_3339 | 1 | 0 | 0% | 🔴 |
| sql_standard | 1 | 80 | 100% | 🟢 |
| aba_routing | 1 | 80 | 100% | 🟢 |
| bsb | 1 | 80 | 100% | 🟢 |
| swift_bic | 1 | 80 | 100% | 🟢 |
| bitcoin_address | 1 | 25 | 100% | 🟢 |
| credit_card_number | 1 | 80 | 100% | 🟢 |
| cusip | 1 | 25 | 100% | 🟢 |
| figi | 1 | 80 | 100% | 🟢 |
| isin | 1 | 100 | 100% | 🟢 |
| lei | 1 | 25 | 100% | 🟢 |
| coordinates | 1 | 100 | 100% | 🟢 |
| dms | 1 | 80 | 100% | 🟢 |
| mgrs | 1 | 80 | 100% | 🟢 |
| plus_code | 1 | 80 | 100% | 🟢 |
| wkt | 1 | 80 | 100% | 🟢 |
| h3 | 1 | 80 | 100% | 🟢 |
| hs_code | 1 | 80 | 100% | 🟢 |
| iata_code | 1 | 7698 | 100% | 🟢 |
| iso6346 | 1 | 80 | 100% | 🟢 |
| unlocode | 1 | 80 | 100% | 🟢 |
| orcid | 1 | 80 | 100% | 🟢 |
| issn | 1 | 80 | 100% | 🟢 |
| abn | 1 | 80 | 100% | 🟢 |
| ein | 1 | 80 | 100% | 🟢 |
| eu_vat | 1 | 80 | 100% | 🟢 |
| pan_india | 1 | 80 | 100% | 🟢 |
| vin | 1 | 80 | 100% | 🟢 |
| cpt | 1 | 80 | 100% | 🟢 |
| dea_number | 1 | 249 | 100% | 🟢 |
| hcpcs | 1 | 80 | 100% | 🟢 |
| loinc | 1 | 80 | 100% | 🟢 |
| npi | 1 | 60 | 100% | 🟢 |
| username | 1 | 25 | 100% | 🟢 |
| binary | 1 | 891 | 100% | 🟢 |
| file_size | 1 | 25 | 100% | 🟢 |
| color_hex | 1 | 80 | 100% | 🟢 |
| color_hsl | 1 | 80 | 100% | 🟢 |
| alphanumeric_id | 1 | 50 | 100% | 🟢 |
| increment | 1 | 891 | 100% | 🟢 |
| numeric_code | 1 | 247 | 100% | 🟢 |
| decimal_number_comma | 1 | 25 | 100% | 🟢 |
| scientific_notation | 1 | 100 | 0% | 🔴 |
| cas_number | 1 | 80 | 100% | 🟢 |
| inchi | 1 | 80 | 100% | 🟢 |
| aws_arn | 1 | 80 | 100% | 🟢 |
| s3_uri | 1 | 80 | 100% | 🟢 |
| token_urlsafe | 1 | 80 | 100% | 🟢 |
| calver | 1 | 25 | 100% | 🟢 |
| docker_ref | 1 | 80 | 100% | 🟢 |
| tsid | 1 | 80 | 100% | 🟢 |
| ulid | 1 | 80 | 100% | 🟢 |
| cidr | 1 | 80 | 100% | 🟢 |
| http_method | 1 | 100 | 100% | 🟢 |
| ip_v6 | 1 | 25 | 100% | 🟢 |
| mac_address | 1 | 80 | 100% | 🟢 |
| top_level_domain | 1 | 14132 | 100% | 🟢 |
| urn | 1 | 80 | 100% | 🟢 |

### Below Target (<95%)

| Dataset | Column | Type | Format | Success Rate |
|---|---|---|---|---|
| covid_timeseries | Date | iso_8601 | `` | 0% 🔴 |
| ecommerce_orders | order_date | iso_8601 | `` | 0% 🔴 |
| financial_data | date | iso_8601 | `` | 0% 🔴 |
| datetime_formats | iso_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats | us_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats | eu_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats | year | compact_ym | `` | 0% 🔴 |
| medical_records | visit_date | iso_8601 | `` | 0% 🔴 |
| network_logs | user_agent | mdy_12h | `` | 0% 🔴 |
| multilingual | date | iso_8601 | `` | 0% 🔴 |
| sports_events | event_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | eu_dot_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | abbreviated_month_date | abbrev_month_no_comma | `` | 0% 🔴 |
| datetime_formats_extended | long_full_month_date | full_month_no_comma | `` | 0% 🔴 |
| datetime_formats_extended | american_timestamp | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | european_timestamp | iso_8601 | `` | 0% 🔴 |
| ecommerce_orders_json | order_date | iso_8601 | `` | 0% 🔴 |
| weather_stations_json | observation_date | iso_8601 | `` | 0% 🔴 |
| earthquakes_2024 | horizontalError | dmy_short_dot | `` | 0% 🔴 |
| datetime_coverage | rfc_3339 | rfc_3339 | `` | 0% 🔴 |
| datetime_coverage | dmy_dash | iso | `` | 0% 🔴 |
| datetime_coverage | iso_week | iso | `` | 0% 🔴 |
| datetime_coverage | clf_timestamp | iso_8601 | `` | 0% 🔴 |
| datetime_coverage | syslog_bsd | clf | `` | 0% 🔴 |
| datetime_coverage | mdy_dash | iso | `` | 0% 🔴 |
| financial_data | market_cap | longitude | `` | 0% 🔴 |
| codes_and_ids | iban | snowflake_id | `` | 0% 🔴 |
| medical_records | blood_pressure | decimal_number | `` | 0% 🔴 |
| sports_events | event_id | scientific_notation | `` | 0% 🔴 |
| datetime_coverage | quarter | quarter | `` | 0% 🔴 |
| codes_and_ids | semantic_version | dmy_short_dot | `` | 28.7% 🔴 |
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
