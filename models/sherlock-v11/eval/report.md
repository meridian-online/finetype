# FineType Evaluation Report

**Generated:** 2026-04-12 18:34

## Headline Metrics

| Metric | Value | Status |
|---|---|---|
| Profile label accuracy | 203/227 (89.4%) | 🟡 |
| Profile domain accuracy | 210/227 (92.5%) | 🟡 |
| Actionability (datetime) | 544403/545301 (99.8%) | 🟢 |
| Columns with >95% parse rate | 313/330 | |
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

**Label accuracy:** 203/227 (89.4%)
**Domain accuracy:** 210/227 (92.5%)

### Misclassifications

| Dataset | Column | Predicted | Expected | Confidence |
|---|---|---|---|---|
| ecommerce_orders | phone | ssn | phone_number | 1.00 |
| airports | icao | unlocode | icao_code | 1.00 |
| new_technology | git_sha | tsid | hash | 1.00 |
| people_directory | phone | ssn | phone_number | 1.00 |
| books_catalog | author | email_display | full_name | 1.00 |
| tech_systems | server_hostname | docker_ref | hostname | 0.99 |
| tech_systems | port | ean | integer_number | 0.99 |
| earthquakes_2024 | id | geohash | alphanumeric_id | 0.99 |
| codes_and_ids | sha256 | ethereum_address | hash | 0.96 |
| datetime_formats_extended | eu_dot_date | iso_8601 | dmy_dot | 0.95 |
| new_geography | geojson | plain_text | json | 0.89 |
| network_logs | user_agent | docker_ref | user_agent | 0.83 |
| datetime_formats | year | compact_ym | year | 0.83 |
| server_logs_json | status_code | postal_code | integer_number | 0.81 |
| finance_coverage | cusip | isrc | cusip | 0.74 |
| network_logs | status_code | bsb | integer_number | 0.69 |
| finance_coverage | bitcoin_address | full_address | bitcoin_address | 0.68 |
| earthquakes_2024 | depthError | latitude | decimal_number | 0.61 |
| ecommerce_orders | tracking_url | docker_ref | url | 0.60 |
| representation_coverage | scientific_notation | decimal_number | scientific_notation | 0.59 |
| technology_coverage | ip_v6 | ip_v4 | ip_v6 | 0.50 |
| multilingual | locale | alphanumeric_id | locale_code | 0.40 |
| tech_systems | user_agent | jwt | user_agent | 0.37 |
| server_logs_json | method | iata_code | http_method | 0.29 |

## Precision Per Type (Profile Eval)

| Predicted Type | Predicted | Correct | Precision | Status |
|---|---|---|---|---|
| decimal_number | 23 | 22 | 95.7% | 🟢 |
| country | 8 | 8 | 100% | 🟢 |
| ip_v4 | 6 | 5 | 83.3% | 🟡 |
| latitude | 6 | 5 | 83.3% | 🟡 |
| state | 5 | 5 | 100% | 🟢 |
| full_address | 5 | 4 | 80% | 🟡 |
| country_code | 5 | 5 | 100% | 🟢 |
| postal_code | 5 | 4 | 80% | 🟡 |
| longitude | 5 | 5 | 100% | 🟢 |
| city | 5 | 5 | 100% | 🟢 |
| full_name | 4 | 4 | 100% | 🟢 |
| entity_name | 4 | 4 | 100% | 🟢 |
| docker_ref | 4 | 1 | 25% | 🔴 |
| url | 4 | 4 | 100% | 🟢 |
| email | 4 | 4 | 100% | 🟢 |
| terms | 4 | 4 | 100% | 🟢 |
| percentage | 3 | 3 | 100% | 🟢 |
| gender | 3 | 3 | 100% | 🟢 |
| iso_8601 | 3 | 2 | 66.7% | 🔴 |
| ssn | 3 | 1 | 33.3% | 🔴 |
| uuid | 3 | 3 | 100% | 🟢 |
| jwt | 2 | 1 | 50% | 🔴 |
| integer_number | 2 | 2 | 100% | 🟢 |
| tsid | 2 | 1 | 50% | 🔴 |
| mdy_dash | 2 | 2 | 100% | 🟢 |
| iso_8601_milliseconds | 2 | 2 | 100% | 🟢 |
| categorical | 2 | 2 | 100% | 🟢 |
| unlocode | 2 | 1 | 50% | 🔴 |
| clf | 2 | 2 | 100% | 🟢 |
| height | 2 | 2 | 100% | 🟢 |
| iata_code | 2 | 1 | 50% | 🔴 |
| ean | 2 | 1 | 50% | 🔴 |
| alphanumeric_id | 2 | 1 | 50% | 🔴 |
| last_name | 2 | 2 | 100% | 🟢 |
| phone_number | 2 | 2 | 100% | 🟢 |
| weight | 2 | 2 | 100% | 🟢 |
| utc | 2 | 2 | 100% | 🟢 |
| geohash | 2 | 1 | 50% | 🔴 |
| first_name | 2 | 2 | 100% | 🟢 |
| email_display | 2 | 1 | 50% | 🔴 |
| bsb | 2 | 1 | 50% | 🔴 |
| currency_code | 2 | 2 | 100% | 🟢 |
| iana | 2 | 2 | 100% | 🟢 |
| isrc | 2 | 1 | 50% | 🔴 |
| cpt | 1 | 1 | 100% | 🟢 |
| iso_space_zulu | 1 | 1 | 100% | 🟢 |
| vin | 1 | 1 | 100% | 🟢 |
| isin | 1 | 1 | 100% | 🟢 |
| mgrs | 1 | 1 | 100% | 🟢 |
| day_of_week | 1 | 1 | 100% | 🟢 |
| fiscal_year | 1 | 1 | 100% | 🟢 |
| compact_ym | 1 | 0 | 0% | 🔴 |
| cas_number | 1 | 1 | 100% | 🟢 |
| ulid | 1 | 1 | 100% | 🟢 |
| color_hsl | 1 | 1 | 100% | 🟢 |
| aws_arn | 1 | 1 | 100% | 🟢 |
| lei | 1 | 1 | 100% | 🟢 |
| dmy_hm | 1 | 1 | 100% | 🟢 |
| credit_card_number | 1 | 1 | 100% | 🟢 |
| month_name | 1 | 1 | 100% | 🟢 |
| aba_routing | 1 | 1 | 100% | 🟢 |
| ymd_dot | 1 | 1 | 100% | 🟢 |
| sql_standard | 1 | 1 | 100% | 🟢 |
| rfc_2822 | 1 | 1 | 100% | 🟢 |
| phone_e164 | 1 | 1 | 100% | 🟢 |
| h3 | 1 | 1 | 100% | 🟢 |
| upc | 1 | 1 | 100% | 🟢 |
| orcid | 1 | 1 | 100% | 🟢 |
| full_month_no_comma | 1 | 1 | 100% | 🟢 |
| ein | 1 | 1 | 100% | 🟢 |
| compact_dmy | 1 | 1 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 1 | 100% | 🟢 |
| dms | 1 | 1 | 100% | 🟢 |
| calver | 1 | 1 | 100% | 🟢 |
| smiles | 1 | 1 | 100% | 🟢 |
| plain_text | 1 | 0 | 0% | 🔴 |
| s3_uri | 1 | 1 | 100% | 🟢 |
| issn | 1 | 1 | 100% | 🟢 |
| inchi | 1 | 1 | 100% | 🟢 |
| iso6346 | 1 | 1 | 100% | 🟢 |
| loinc | 1 | 1 | 100% | 🟢 |
| icd10 | 1 | 1 | 100% | 🟢 |
| snowflake_id | 1 | 1 | 100% | 🟢 |
| figi | 1 | 1 | 100% | 🟢 |
| hm_12h | 1 | 1 | 100% | 🟢 |
| plus_code | 1 | 1 | 100% | 🟢 |
| swift_bic | 1 | 1 | 100% | 🟢 |
| urn | 1 | 1 | 100% | 🟢 |
| mdy_12h | 1 | 1 | 100% | 🟢 |
| iso_week | 1 | 1 | 100% | 🟢 |
| npi | 1 | 1 | 100% | 🟢 |
| eu_vat | 1 | 1 | 100% | 🟢 |
| locale_code | 1 | 1 | 100% | 🟢 |
| hcpcs | 1 | 1 | 100% | 🟢 |
| abn | 1 | 1 | 100% | 🟢 |
| hs_code | 1 | 1 | 100% | 🟢 |
| iso | 1 | 1 | 100% | 🟢 |
| pan_india | 1 | 1 | 100% | 🟢 |
| iso_8601_offset | 1 | 1 | 100% | 🟢 |
| ethereum_address | 1 | 0 | 0% | 🔴 |
| decimal_number_comma | 1 | 1 | 100% | 🟢 |
| year | 1 | 1 | 100% | 🟢 |
| cidr | 1 | 1 | 100% | 🟢 |
| data_uri | 1 | 1 | 100% | 🟢 |
| hms_12h | 1 | 1 | 100% | 🟢 |
| compact_ymd | 1 | 1 | 100% | 🟢 |
| mac_address | 1 | 1 | 100% | 🟢 |
| user_agent | 1 | 1 | 100% | 🟢 |
| file_size | 1 | 1 | 100% | 🟢 |
| binary | 1 | 1 | 100% | 🟢 |
| wkt | 1 | 1 | 100% | 🟢 |
| quarter | 1 | 1 | 100% | 🟢 |
| username | 1 | 1 | 100% | 🟢 |

## Actionability Evaluation

Can analysts safely TRY_CAST using FineType's format_string predictions?
**Target:** >95% success rate for datetime types

### By Type

| Type | Columns | Values | Success Rate | Status |
|---|---|---|---|---|
| decimal_number | 25 | 100048 | 99.8% | 🟢 |
| integer_number | 21 | 49193 | 100% | 🟢 |
| iso | 10 | 1720 | 96.5% | 🟢 |
| amount | 10 | 1676 | 100% | 🟢 |
| country | 9 | 56676 | 100% | 🟢 |
| categorical | 9 | 1554 | 100% | 🟢 |
| iso_8601 | 7 | 440 | 76.1% | 🔴 |
| postal_code | 6 | 445 | 100% | 🟢 |
| latitude | 6 | 36172 | 100% | 🟢 |
| country_code | 6 | 967 | 100% | 🟢 |
| entity_name | 6 | 8043 | 100% | 🟢 |
| ip_v4 | 6 | 355 | 100% | 🟢 |
| full_address | 5 | 22015 | 100% | 🟢 |
| longitude | 5 | 22040 | 100% | 🟢 |
| city | 5 | 41106 | 100% | 🟢 |
| url | 5 | 325 | 100% | 🟢 |
| currency_code | 4 | 250 | 100% | 🟢 |
| ssn | 4 | 380 | 100% | 🟢 |
| email | 4 | 285 | 100% | 🟢 |
| full_name | 4 | 1111 | 100% | 🟢 |
| terms | 4 | 285 | 100% | 🟢 |
| ordinal | 4 | 1867 | 100% | 🟢 |
| alphanumeric_id | 4 | 235 | 100% | 🟢 |
| docker_ref | 4 | 360 | 100% | 🟢 |
| gender | 3 | 1051 | 100% | 🟢 |
| mime_type | 3 | 205 | 100% | 🟢 |
| increment | 3 | 34228 | 100% | 🟢 |
| uuid | 3 | 260 | 100% | 🟢 |
| percentage | 3 | 250 | 100% | 🟢 |
| whitespace_separated | 2 | 120 | 100% | 🟢 |
| periodicity | 2 | 200 | 100% | 🟢 |
| dmy_short_dot | 2 | 160 | 61.3% | 🔴 |
| mdy_dash | 2 | 50 | 68% | 🔴 |
| ymd_slash | 2 | 160 | 0% | 🔴 |
| iana | 2 | 7778 | 100% | 🟢 |
| utc | 2 | 7778 | 100% | 🟢 |
| clf | 2 | 50 | 0% | 🔴 |
| iso_8601_milliseconds | 2 | 28264 | 100% | 🟢 |
| bsb | 2 | 180 | 100% | 🟢 |
| credit_card_number | 2 | 160 | 100% | 🟢 |
| isin | 2 | 125 | 100% | 🟢 |
| geohash | 2 | 14212 | 100% | 🟢 |
| continent | 2 | 28264 | 100% | 🟢 |
| region | 2 | 7749 | 100% | 🟢 |
| iata_code | 2 | 7723 | 100% | 🟢 |
| unlocode | 2 | 7778 | 100% | 🟢 |
| ean | 2 | 160 | 100% | 🟢 |
| isbn | 2 | 140 | 100% | 🟢 |
| isrc | 2 | 105 | 100% | 🟢 |
| pan_india | 2 | 140 | 100% | 🟢 |
| icd10 | 2 | 140 | 100% | 🟢 |
| email_display | 2 | 140 | 100% | 🟢 |
| first_name | 2 | 160 | 100% | 🟢 |
| height | 2 | 160 | 100% | 🟢 |
| last_name | 2 | 160 | 100% | 🟢 |
| phone_number | 2 | 120 | 100% | 🟢 |
| weight | 2 | 160 | 100% | 🟢 |
| measurement_unit | 2 | 28264 | 100% | 🟢 |
| smiles | 2 | 130 | 100% | 🟢 |
| jwt | 2 | 160 | 100% | 🟢 |
| tsid | 2 | 160 | 100% | 🟢 |
| query_string | 1 | 100 | 100% | 🟢 |
| day_of_week | 1 | 80 | 100% | 🟢 |
| month_name | 1 | 80 | 100% | 🟢 |
| year | 1 | 60 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 80 | 0% | 🔴 |
| compact_dmy | 1 | 25 | 100% | 🟢 |
| compact_ym | 1 | 80 | 0% | 🔴 |
| compact_ymd | 1 | 25 | 100% | 🟢 |
| full_month_no_comma | 1 | 80 | 0% | 🔴 |
| iso_week | 1 | 25 | 100% | 🟢 |
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
| dmy_hm | 1 | 80 | 100% | 🟢 |
| iso_8601_offset | 1 | 25 | 100% | 🟢 |
| iso_space_zulu | 1 | 25 | 100% | 🟢 |
| mdy_12h | 1 | 80 | 100% | 🟢 |
| rfc_2822 | 1 | 80 | 100% | 🟢 |
| sql_standard | 1 | 80 | 100% | 🟢 |
| aba_routing | 1 | 80 | 100% | 🟢 |
| swift_bic | 1 | 80 | 100% | 🟢 |
| ethereum_address | 1 | 80 | 100% | 🟢 |
| amount_nodecimal | 1 | 60 | 66.7% | 🔴 |
| figi | 1 | 80 | 100% | 🟢 |
| lei | 1 | 25 | 100% | 🟢 |
| coordinates | 1 | 100 | 100% | 🟢 |
| dms | 1 | 80 | 100% | 🟢 |
| mgrs | 1 | 80 | 100% | 🟢 |
| plus_code | 1 | 80 | 100% | 🟢 |
| wkt | 1 | 80 | 100% | 🟢 |
| h3 | 1 | 80 | 100% | 🟢 |
| hs_code | 1 | 80 | 100% | 🟢 |
| icao_code | 1 | 100 | 100% | 🟢 |
| iso6346 | 1 | 80 | 100% | 🟢 |
| orcid | 1 | 80 | 100% | 🟢 |
| issn | 1 | 80 | 100% | 🟢 |
| upc | 1 | 80 | 100% | 🟢 |
| abn | 1 | 80 | 100% | 🟢 |
| ein | 1 | 80 | 100% | 🟢 |
| eu_vat | 1 | 80 | 100% | 🟢 |
| vin | 1 | 80 | 100% | 🟢 |
| cpt | 1 | 80 | 100% | 🟢 |
| dea_number | 1 | 249 | 100% | 🟢 |
| hcpcs | 1 | 80 | 100% | 🟢 |
| loinc | 1 | 80 | 100% | 🟢 |
| npi | 1 | 60 | 100% | 🟢 |
| phone_e164 | 1 | 80 | 100% | 🟢 |
| username | 1 | 25 | 100% | 🟢 |
| binary | 1 | 891 | 100% | 🟢 |
| file_size | 1 | 25 | 100% | 🟢 |
| color_hex | 1 | 80 | 100% | 🟢 |
| color_hsl | 1 | 80 | 100% | 🟢 |
| numeric_code | 1 | 247 | 100% | 🟢 |
| decimal_number_comma | 1 | 25 | 100% | 🟢 |
| cas_number | 1 | 80 | 100% | 🟢 |
| inchi | 1 | 80 | 100% | 🟢 |
| plain_text | 1 | 80 | 100% | 🟢 |
| word | 1 | 14132 | 100% | 🟢 |
| aws_arn | 1 | 80 | 100% | 🟢 |
| s3_uri | 1 | 80 | 100% | 🟢 |
| locale_code | 1 | 80 | 100% | 🟢 |
| token_urlsafe | 1 | 80 | 100% | 🟢 |
| calver | 1 | 25 | 100% | 🟢 |
| snowflake_id | 1 | 80 | 100% | 🟢 |
| ulid | 1 | 80 | 100% | 🟢 |
| cidr | 1 | 80 | 100% | 🟢 |
| data_uri | 1 | 80 | 100% | 🟢 |
| http_method | 1 | 100 | 100% | 🟢 |
| mac_address | 1 | 80 | 100% | 🟢 |
| urn | 1 | 80 | 100% | 🟢 |
| user_agent | 1 | 25 | 100% | 🟢 |

### Below Target (<95%)

| Dataset | Column | Type | Format | Success Rate |
|---|---|---|---|---|
| datetime_formats | us_date | ymd_slash | `` | 0% 🔴 |
| datetime_formats | eu_date | ymd_slash | `` | 0% 🔴 |
| datetime_formats | year | compact_ym | `` | 0% 🔴 |
| multilingual | date | iso | `` | 0% 🔴 |
| datetime_formats_extended | eu_dot_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | abbreviated_month_date | abbrev_month_no_comma | `` | 0% 🔴 |
| datetime_formats_extended | long_full_month_date | full_month_no_comma | `` | 0% 🔴 |
| ecommerce_orders_json | order_date | iso_8601 | `` | 0% 🔴 |
| datetime_coverage | clf_timestamp | clf | `` | 0% 🔴 |
| datetime_coverage | syslog_bsd | clf | `` | 0% 🔴 |
| financial_data | market_cap | decimal_number | `` | 0% 🔴 |
| medical_records | blood_pressure | decimal_number | `` | 0% 🔴 |
| datetime_coverage | quarter | quarter | `` | 0% 🔴 |
| codes_and_ids | semantic_version | dmy_short_dot | `` | 28.7% 🔴 |
| datetime_coverage | dmy_dash | mdy_dash | `` | 36% 🔴 |
| multilingual | price | amount_nodecimal | `` | 66.7% 🔴 |
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
