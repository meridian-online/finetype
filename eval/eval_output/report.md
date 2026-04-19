# FineType Evaluation Report

**Generated:** 2026-04-20 06:35

## Headline Metrics

| Metric | Value | Status |
|---|---|---|
| Profile label accuracy | 235/242 (97.1%) | 🟢 |
| Profile domain accuracy | 233/242 (96.3%) | 🟢 |
| Actionability (datetime) | 578862/578924 (100%) | 🟢 |
| Columns with >95% parse rate | 329/333 | |
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

**Label accuracy:** 235/242 (97.1%)
**Domain accuracy:** 233/242 (96.3%)

### Misclassifications

| Dataset | Column | Predicted | Expected | Confidence |
|---|---|---|---|---|
| tech_systems | user_agent | jwt | user_agent | 1.00 |
| new_geography | geojson | plain_text | json | 0.99 |
| people_directory | phone | ssn | phone_number | 0.97 |
| earthquakes_2024 | id | username | alphanumeric_id | 0.86 |
| datetime_coverage | fiscal_year | year | fiscal_year | 0.50 |
| network_logs | user_agent | docker_ref | user_agent | 0.42 |
| multilingual | locale | categorical | locale_code | 0.26 |

## Precision Per Type (Profile Eval)

| Predicted Type | Predicted | Correct | Precision | Status |
|---|---|---|---|---|
| decimal_number | 23 | 23 | 100% | 🟢 |
| iso | 9 | 9 | 100% | 🟢 |
| country | 8 | 8 | 100% | 🟢 |
| full_name | 5 | 5 | 100% | 🟢 |
| url | 5 | 5 | 100% | 🟢 |
| longitude | 5 | 5 | 100% | 🟢 |
| integer_number | 5 | 5 | 100% | 🟢 |
| country_code | 5 | 5 | 100% | 🟢 |
| city | 5 | 5 | 100% | 🟢 |
| latitude | 5 | 5 | 100% | 🟢 |
| full_address | 4 | 4 | 100% | 🟢 |
| email | 4 | 4 | 100% | 🟢 |
| ip_v4 | 4 | 4 | 100% | 🟢 |
| iso_8601_milliseconds | 4 | 4 | 100% | 🟢 |
| postal_code | 4 | 4 | 100% | 🟢 |
| entity_name | 4 | 4 | 100% | 🟢 |
| terms | 4 | 4 | 100% | 🟢 |
| gender | 3 | 3 | 100% | 🟢 |
| percentage | 3 | 3 | 100% | 🟢 |
| year | 3 | 2 | 66.7% | 🔴 |
| phone_number | 3 | 3 | 100% | 🟢 |
| uuid | 3 | 3 | 100% | 🟢 |
| ssn | 3 | 2 | 66.7% | 🔴 |
| utc | 2 | 2 | 100% | 🟢 |
| weight | 2 | 2 | 100% | 🟢 |
| username | 2 | 1 | 50% | 🔴 |
| http_method | 2 | 2 | 100% | 🟢 |
| last_name | 2 | 2 | 100% | 🟢 |
| icd10 | 2 | 2 | 100% | 🟢 |
| hash | 2 | 2 | 100% | 🟢 |
| first_name | 2 | 2 | 100% | 🟢 |
| currency_code | 2 | 2 | 100% | 🟢 |
| dmy_dash | 2 | 2 | 100% | 🟢 |
| iana | 2 | 2 | 100% | 🟢 |
| height | 2 | 2 | 100% | 🟢 |
| docker_ref | 2 | 1 | 50% | 🔴 |
| region | 2 | 2 | 100% | 🟢 |
| categorical | 2 | 1 | 50% | 🔴 |
| jwt | 2 | 1 | 50% | 🔴 |
| continent | 2 | 2 | 100% | 🟢 |
| sql_standard | 1 | 1 | 100% | 🟢 |
| compact_ymd | 1 | 1 | 100% | 🟢 |
| phone_e164 | 1 | 1 | 100% | 🟢 |
| mdy_12h | 1 | 1 | 100% | 🟢 |
| mac_address | 1 | 1 | 100% | 🟢 |
| eu_vat | 1 | 1 | 100% | 🟢 |
| snowflake_id | 1 | 1 | 100% | 🟢 |
| swift_bic | 1 | 1 | 100% | 🟢 |
| inchi | 1 | 1 | 100% | 🟢 |
| hms_12h | 1 | 1 | 100% | 🟢 |
| ymd_dot | 1 | 1 | 100% | 🟢 |
| iso_8601_offset | 1 | 1 | 100% | 🟢 |
| long_full_month | 1 | 1 | 100% | 🟢 |
| dmy_dot | 1 | 1 | 100% | 🟢 |
| ean | 1 | 1 | 100% | 🟢 |
| month_name | 1 | 1 | 100% | 🟢 |
| file_size | 1 | 1 | 100% | 🟢 |
| smiles | 1 | 1 | 100% | 🟢 |
| binary | 1 | 1 | 100% | 🟢 |
| cpt | 1 | 1 | 100% | 🟢 |
| hostname | 1 | 1 | 100% | 🟢 |
| mgrs | 1 | 1 | 100% | 🟢 |
| isrc | 1 | 1 | 100% | 🟢 |
| issn | 1 | 1 | 100% | 🟢 |
| aws_arn | 1 | 1 | 100% | 🟢 |
| quarter | 1 | 1 | 100% | 🟢 |
| rfc_3339 | 1 | 1 | 100% | 🟢 |
| calver | 1 | 1 | 100% | 🟢 |
| icao_code | 1 | 1 | 100% | 🟢 |
| hs_code | 1 | 1 | 100% | 🟢 |
| abbreviated_month | 1 | 1 | 100% | 🟢 |
| ip_v6 | 1 | 1 | 100% | 🟢 |
| compact_dmy | 1 | 1 | 100% | 🟢 |
| email_display | 1 | 1 | 100% | 🟢 |
| user_agent | 1 | 1 | 100% | 🟢 |
| credit_card_number | 1 | 1 | 100% | 🟢 |
| rfc_2822 | 1 | 1 | 100% | 🟢 |
| hm_12h | 1 | 1 | 100% | 🟢 |
| tsid | 1 | 1 | 100% | 🟢 |
| iso_8601 | 1 | 1 | 100% | 🟢 |
| day_of_week | 1 | 1 | 100% | 🟢 |
| iso_week | 1 | 1 | 100% | 🟢 |
| dmy_hm | 1 | 1 | 100% | 🟢 |
| decimal_number_comma | 1 | 1 | 100% | 🟢 |
| ip_v4_with_port | 1 | 1 | 100% | 🟢 |
| clf | 1 | 1 | 100% | 🟢 |
| pan_india | 1 | 1 | 100% | 🟢 |
| iban | 1 | 1 | 100% | 🟢 |
| isin | 1 | 1 | 100% | 🟢 |
| figi | 1 | 1 | 100% | 🟢 |
| scientific_notation | 1 | 1 | 100% | 🟢 |
| measurement_unit | 1 | 1 | 100% | 🟢 |
| lei | 1 | 1 | 100% | 🟢 |
| wkt | 1 | 1 | 100% | 🟢 |
| geohash | 1 | 1 | 100% | 🟢 |
| h3 | 1 | 1 | 100% | 🟢 |
| iso6346 | 1 | 1 | 100% | 🟢 |
| cidr | 1 | 1 | 100% | 🟢 |
| data_uri | 1 | 1 | 100% | 🟢 |
| ein | 1 | 1 | 100% | 🟢 |
| alphanumeric_id | 1 | 1 | 100% | 🟢 |
| dms | 1 | 1 | 100% | 🟢 |
| hcpcs | 1 | 1 | 100% | 🟢 |
| bsb | 1 | 1 | 100% | 🟢 |
| aba_routing | 1 | 1 | 100% | 🟢 |
| loinc | 1 | 1 | 100% | 🟢 |
| cusip | 1 | 1 | 100% | 🟢 |
| vin | 1 | 1 | 100% | 🟢 |
| mdy_slash | 1 | 1 | 100% | 🟢 |
| dmy_slash | 1 | 1 | 100% | 🟢 |
| upc | 1 | 1 | 100% | 🟢 |
| s3_uri | 1 | 1 | 100% | 🟢 |
| bitcoin_address | 1 | 1 | 100% | 🟢 |
| syslog_bsd | 1 | 1 | 100% | 🟢 |
| locale_code | 1 | 1 | 100% | 🟢 |
| plain_text | 1 | 0 | 0% | 🔴 |
| unlocode | 1 | 1 | 100% | 🟢 |
| urn | 1 | 1 | 100% | 🟢 |
| orcid | 1 | 1 | 100% | 🟢 |
| npi | 1 | 1 | 100% | 🟢 |
| ulid | 1 | 1 | 100% | 🟢 |
| iata_code | 1 | 1 | 100% | 🟢 |
| state | 1 | 1 | 100% | 🟢 |
| plus_code | 1 | 1 | 100% | 🟢 |
| cas_number | 1 | 1 | 100% | 🟢 |
| abn | 1 | 1 | 100% | 🟢 |
| color_hsl | 1 | 1 | 100% | 🟢 |

## Actionability Evaluation

Can analysts safely TRY_CAST using FineType's format_string predictions?
**Target:** >95% success rate for datetime types

### By Type

| Type | Columns | Values | Success Rate | Status |
|---|---|---|---|---|
| integer_number | 24 | 96443 | 100% | 🟢 |
| decimal_number | 23 | 99940 | 100% | 🟢 |
| amount | 11 | 1736 | 100% | 🟢 |
| categorical | 11 | 1764 | 100% | 🟢 |
| iso | 9 | 1660 | 100% | 🟢 |
| country | 8 | 42544 | 100% | 🟢 |
| entity_name | 8 | 8352 | 100% | 🟢 |
| city | 6 | 55238 | 100% | 🟢 |
| country_code | 6 | 967 | 100% | 🟢 |
| url | 6 | 425 | 100% | 🟢 |
| postal_code | 5 | 420 | 100% | 🟢 |
| latitude | 5 | 22040 | 100% | 🟢 |
| longitude | 5 | 22040 | 100% | 🟢 |
| full_name | 5 | 1171 | 100% | 🟢 |
| alphanumeric_id | 5 | 335 | 100% | 🟢 |
| measurement_unit | 5 | 56578 | 100% | 🟢 |
| iso_8601 | 4 | 235 | 100% | 🟢 |
| iso_8601_milliseconds | 4 | 28389 | 100% | 🟢 |
| currency_code | 4 | 250 | 100% | 🟢 |
| full_address | 4 | 21990 | 100% | 🟢 |
| email | 4 | 285 | 100% | 🟢 |
| terms | 4 | 285 | 100% | 🟢 |
| ordinal | 4 | 1867 | 100% | 🟢 |
| ip_v4 | 4 | 305 | 100% | 🟢 |
| year | 3 | 140 | 100% | 🟢 |
| ssn | 3 | 280 | 100% | 🟢 |
| gender | 3 | 1051 | 100% | 🟢 |
| phone_number | 3 | 220 | 100% | 🟢 |
| uuid | 3 | 260 | 100% | 🟢 |
| percentage | 3 | 250 | 100% | 🟢 |
| dmy_dash | 2 | 50 | 68% | 🔴 |
| iana | 2 | 7778 | 100% | 🟢 |
| utc | 2 | 7778 | 100% | 🟢 |
| continent | 2 | 494 | 100% | 🟢 |
| region | 2 | 33214 | 100% | 🟢 |
| icao_code | 2 | 7798 | 100% | 🟢 |
| ean | 2 | 180 | 100% | 🟢 |
| isbn | 2 | 140 | 100% | 🟢 |
| icd10 | 2 | 140 | 100% | 🟢 |
| first_name | 2 | 160 | 100% | 🟢 |
| height | 2 | 160 | 100% | 🟢 |
| last_name | 2 | 160 | 100% | 🟢 |
| username | 2 | 14157 | 100% | 🟢 |
| weight | 2 | 160 | 100% | 🟢 |
| mime_type | 2 | 180 | 100% | 🟢 |
| numeric_code | 2 | 494 | 100% | 🟢 |
| smiles | 2 | 130 | 100% | 🟢 |
| plain_text | 2 | 105 | 100% | 🟢 |
| word | 2 | 21830 | 100% | 🟢 |
| hash | 2 | 160 | 100% | 🟢 |
| jwt | 2 | 160 | 100% | 🟢 |
| docker_ref | 2 | 180 | 100% | 🟢 |
| http_method | 2 | 125 | 100% | 🟢 |
| whitespace_separated | 1 | 60 | 100% | 🟢 |
| query_string | 1 | 100 | 100% | 🟢 |
| day_of_week | 1 | 80 | 100% | 🟢 |
| month_name | 1 | 80 | 100% | 🟢 |
| abbreviated_month | 1 | 80 | 100% | 🟢 |
| compact_dmy | 1 | 25 | 100% | 🟢 |
| compact_ymd | 1 | 25 | 100% | 🟢 |
| dmy_dot | 1 | 80 | 100% | 🟢 |
| dmy_short_dot | 1 | 80 | 93.8% | 🟡 |
| dmy_slash | 1 | 80 | 100% | 🟢 |
| iso_week | 1 | 25 | 100% | 🟢 |
| long_full_month | 1 | 80 | 100% | 🟢 |
| mdy_slash | 1 | 80 | 100% | 🟢 |
| ymd_dot | 1 | 25 | 100% | 🟢 |
| ymd_slash | 1 | 60 | 33.3% | 🔴 |
| iso_8601 | 1 | 80 | 100% | 🟢 |
| unix_milliseconds | 1 | 80 | 100% | 🟢 |
| unix_seconds | 1 | 80 | 100% | 🟢 |
| quarter | 1 | 25 | 100% | 🟢 |
| hm_12h | 1 | 80 | 100% | 🟢 |
| hm_24h | 1 | 100 | 100% | 🟢 |
| hms_12h | 1 | 80 | 100% | 🟢 |
| hms_24h | 1 | 80 | 100% | 🟢 |
| clf | 1 | 25 | 100% | 🟢 |
| dmy_hm | 1 | 80 | 100% | 🟢 |
| iso_8601_offset | 1 | 25 | 100% | 🟢 |
| mdy_12h | 1 | 80 | 100% | 🟢 |
| rfc_2822 | 1 | 80 | 100% | 🟢 |
| rfc_3339 | 1 | 25 | 100% | 🟢 |
| sql_standard | 1 | 80 | 100% | 🟢 |
| syslog_bsd | 1 | 25 | 96% | 🟢 |
| aba_routing | 1 | 80 | 100% | 🟢 |
| bsb | 1 | 80 | 100% | 🟢 |
| iban | 1 | 80 | 100% | 🟢 |
| swift_bic | 1 | 80 | 100% | 🟢 |
| bitcoin_address | 1 | 25 | 100% | 🟢 |
| credit_card_number | 1 | 80 | 100% | 🟢 |
| cusip | 1 | 25 | 100% | 🟢 |
| figi | 1 | 80 | 100% | 🟢 |
| isin | 1 | 25 | 100% | 🟢 |
| lei | 1 | 25 | 100% | 🟢 |
| coordinates | 1 | 100 | 100% | 🟢 |
| dms | 1 | 80 | 100% | 🟢 |
| geohash | 1 | 80 | 100% | 🟢 |
| mgrs | 1 | 80 | 100% | 🟢 |
| plus_code | 1 | 80 | 100% | 🟢 |
| wkt | 1 | 80 | 100% | 🟢 |
| h3 | 1 | 80 | 100% | 🟢 |
| state_code | 1 | 51 | 100% | 🟢 |
| hs_code | 1 | 80 | 100% | 🟢 |
| iata_code | 1 | 7698 | 100% | 🟢 |
| iso6346 | 1 | 80 | 100% | 🟢 |
| unlocode | 1 | 80 | 100% | 🟢 |
| orcid | 1 | 80 | 100% | 🟢 |
| isrc | 1 | 80 | 100% | 🟢 |
| issn | 1 | 80 | 100% | 🟢 |
| upc | 1 | 80 | 100% | 🟢 |
| abn | 1 | 80 | 100% | 🟢 |
| ein | 1 | 80 | 100% | 🟢 |
| eu_vat | 1 | 80 | 100% | 🟢 |
| pan_india | 1 | 80 | 100% | 🟢 |
| vin | 1 | 80 | 100% | 🟢 |
| cpt | 1 | 80 | 100% | 🟢 |
| hcpcs | 1 | 80 | 100% | 🟢 |
| loinc | 1 | 80 | 100% | 🟢 |
| npi | 1 | 60 | 100% | 🟢 |
| email_display | 1 | 80 | 100% | 🟢 |
| phone_e164 | 1 | 80 | 100% | 🟢 |
| binary | 1 | 891 | 100% | 🟢 |
| file_size | 1 | 25 | 100% | 🟢 |
| color_hex | 1 | 80 | 100% | 🟢 |
| color_hsl | 1 | 80 | 100% | 🟢 |
| increment | 1 | 891 | 100% | 🟢 |
| decimal_number_comma | 1 | 25 | 100% | 🟢 |
| scientific_notation | 1 | 25 | 100% | 🟢 |
| si_number | 1 | 100 | 100% | 🟢 |
| cas_number | 1 | 80 | 100% | 🟢 |
| inchi | 1 | 80 | 100% | 🟢 |
| aws_arn | 1 | 80 | 100% | 🟢 |
| s3_uri | 1 | 80 | 100% | 🟢 |
| locale_code | 1 | 80 | 100% | 🟢 |
| token_urlsafe | 1 | 80 | 100% | 🟢 |
| calver | 1 | 25 | 100% | 🟢 |
| version | 1 | 80 | 100% | 🟢 |
| snowflake_id | 1 | 80 | 100% | 🟢 |
| tsid | 1 | 80 | 100% | 🟢 |
| ulid | 1 | 80 | 100% | 🟢 |
| cidr | 1 | 80 | 100% | 🟢 |
| data_uri | 1 | 80 | 100% | 🟢 |
| hostname | 1 | 80 | 100% | 🟢 |
| ip_v4_with_port | 1 | 25 | 100% | 🟢 |
| ip_v6 | 1 | 25 | 100% | 🟢 |
| mac_address | 1 | 80 | 100% | 🟢 |
| urn | 1 | 80 | 100% | 🟢 |
| user_agent | 1 | 25 | 100% | 🟢 |

### Below Target (<95%)

| Dataset | Column | Type | Format | Success Rate |
|---|---|---|---|---|
| datetime_coverage | fiscal_year | year | `` | 0% 🔴 |
| multilingual | date | ymd_slash | `` | 33.3% 🔴 |
| datetime_coverage | mdy_dash | dmy_dash | `` | 36% 🔴 |
| tech_systems | version | dmy_short_dot | `` | 93.8% 🔴 |

## Evaluation Components

| Component | Scope | Target | Status |
|---|---|---|---|
| Profile regression | 242 columns, 35 datasets | No regressions | 🟢 |
| Precision per type | SOTAB/GitTables | 🟢≥95% per type | Run `make eval-sotab-cli` |
| Overcall analysis | SOTAB/GitTables | <5% FP rate | Run `make eval-sotab-cli` |
| Actionability | Profile eval datetime | >95% parse rate | 🟢 |
| Confidence calibration | SOTAB/GitTables | Gap <10pp | Run `make eval-sotab-cli` |
| Domain accuracy | SOTAB format-detectable | >80% | Run `make eval-sotab-cli` |

---
*Generated by eval-report (NNFT-184, Rust port of eval_report.py)*
