# FineType Evaluation Report

**Generated:** 2026-03-25 18:15

## Headline Metrics

| Metric | Value | Status |
|---|---|---|
| Profile label accuracy | 155/190 (81.6%) | 🟡 |
| Profile domain accuracy | 167/190 (87.9%) | 🔴 |
| Actionability (datetime) | 513026/528540 (97.1%) | 🟢 |
| Columns with >95% parse rate | 240/281 | |
| Taxonomy types | 250 | |
| Types with format_string | 66 | |
| Types with validation | 250 | |
| Types with locale validation | 5 | |

## Taxonomy Coverage

| Domain | Types |
|---|---|
| container | 12 |
| datetime | 84 |
| finance | 31 |
| geography | 25 |
| identity | 34 |
| representation | 36 |
| technology | 28 |

## Profile Evaluation

**Label accuracy:** 155/190 (81.6%)
**Domain accuracy:** 167/190 (87.9%)

### Misclassifications

| Dataset | Column | Predicted | Expected | Confidence |
|---|---|---|---|---|
| codes_and_ids | sha256 | git_sha | hash | 1.00 |
| datetime_formats | year | compact_ym | year | 1.00 |
| scientific_measurements | value | hs_code | decimal_number | 1.00 |
| tech_systems | user_agent | jwt | user_agent | 1.00 |
| airports | name | full_address | full_name | 0.99 |
| earthquakes_2024 | rms | hs_code | decimal_number | 0.98 |
| datetime_formats_extended | long_full_month_date | full_month_no_comma | long_full_month | 0.98 |
| earthquakes_2024 | depthError | latitude | decimal_number | 0.98 |
| codes_and_ids | ean | upc | ean | 0.97 |
| ecommerce_orders | phone | ssn | phone_number | 0.97 |
| earthquakes_2024 | dmin | hs_code | decimal_number | 0.95 |
| airports | icao | unlocode | icao_code | 0.92 |
| network_logs | status_code | postal_code | integer_number | 0.90 |
| datetime_formats_extended | eu_dot_date | iso_8601 | dmy_dot | 0.89 |
| datetime_formats_extended | abbreviated_month_date | abbrev_month_no_comma | abbreviated_month | 0.87 |
| tech_systems | port | ean | integer_number | 0.87 |
| people_directory | phone | ssn | phone_number | 0.86 |
| earthquakes_2024 | place | region | full_address | 0.82 |
| medical_records | npi | upc | npi | 0.80 |
| tech_systems | server_hostname | url | hostname | 0.79 |
| earthquakes_2024 | magError | hs_code | decimal_number | 0.79 |
| iris | sepal_length | version | decimal_number | 0.76 |
| earthquakes_2024 | id | geohash | alphanumeric_id | 0.72 |
| iris | sepal_width | version | decimal_number | 0.67 |
| weather_stations_json | wind_speed_kmh | hs_code | decimal_number | 0.60 |
| earthquakes_2024 | depth | longitude | decimal_number | 0.59 |
| network_logs | user_agent | mdy_12h | user_agent | 0.58 |
| earthquakes_2024 | horizontalError | dmy_short_dot | decimal_number | 0.55 |
| weather_stations_json | humidity_pct | hs_code | decimal_number | 0.54 |
| new_identity | upc | ean | upc | 0.52 |
| codes_and_ids | issn | ein | issn | 0.51 |
| api_users_json | phone | abn | phone_number | 0.50 |
| financial_data | pe_ratio | latitude | decimal_number | 0.47 |
| earthquakes_2024 | gap | year | decimal_number | 0.33 |
| scientific_measurements | measurement_unit | categorical | measurement_unit | 0.33 |

## Precision Per Type (Profile Eval)

| Predicted Type | Predicted | Correct | Precision | Status |
|---|---|---|---|---|
| decimal_number | 9 | 9 | 100% | 🟢 |
| country | 8 | 8 | 100% | 🟢 |
| latitude | 7 | 5 | 71.4% | 🔴 |
| longitude | 6 | 5 | 83.3% | 🟡 |
| city | 5 | 5 | 100% | 🟢 |
| state | 5 | 5 | 100% | 🟢 |
| postal_code | 5 | 4 | 80% | 🟡 |
| full_name | 5 | 5 | 100% | 🟢 |
| country_code | 5 | 5 | 100% | 🟢 |
| url | 5 | 4 | 80% | 🟡 |
| hs_code | 4 | 1 | 25% | 🔴 |
| entity_name | 4 | 4 | 100% | 🟢 |
| percentage | 3 | 3 | 100% | 🟢 |
| hs_code | 3 | 0 | 0% | 🔴 |
| uuid | 3 | 3 | 100% | 🟢 |
| gender | 3 | 3 | 100% | 🟢 |
| ssn | 3 | 1 | 33.3% | 🔴 |
| email | 3 | 3 | 100% | 🟢 |
| full_address | 3 | 2 | 66.7% | 🔴 |
| terms | 3 | 3 | 100% | 🟢 |
| ip_v4 | 3 | 3 | 100% | 🟢 |
| unlocode | 2 | 1 | 50% | 🔴 |
| first_name | 2 | 2 | 100% | 🟢 |
| last_name | 2 | 2 | 100% | 🟢 |
| utc | 2 | 2 | 100% | 🟢 |
| height | 2 | 2 | 100% | 🟢 |
| locale_code | 2 | 2 | 100% | 🟢 |
| ean | 2 | 0 | 0% | 🔴 |
| iana | 2 | 2 | 100% | 🟢 |
| geohash | 2 | 1 | 50% | 🔴 |
| iso_8601_milliseconds | 2 | 2 | 100% | 🟢 |
| year | 2 | 1 | 50% | 🔴 |
| upc | 2 | 0 | 0% | 🔴 |
| abn | 2 | 1 | 50% | 🔴 |
| mdy_12h | 2 | 1 | 50% | 🔴 |
| version | 2 | 0 | 0% | 🔴 |
| weight | 2 | 2 | 100% | 🟢 |
| categorical | 2 | 1 | 50% | 🔴 |
| ein | 2 | 1 | 50% | 🔴 |
| jwt | 2 | 1 | 50% | 🔴 |
| git_sha | 2 | 1 | 50% | 🔴 |
| smiles | 1 | 1 | 100% | 🟢 |
| full_month_no_comma | 1 | 0 | 0% | 🔴 |
| aws_arn | 1 | 1 | 100% | 🟢 |
| day_of_week | 1 | 1 | 100% | 🟢 |
| tsid | 1 | 1 | 100% | 🟢 |
| plus_code | 1 | 1 | 100% | 🟢 |
| month_name | 1 | 1 | 100% | 🟢 |
| compact_ym | 1 | 0 | 0% | 🔴 |
| credit_card_number | 1 | 1 | 100% | 🟢 |
| cpt | 1 | 1 | 100% | 🟢 |
| iso_8601 | 1 | 0 | 0% | 🔴 |
| rfc_2822 | 1 | 1 | 100% | 🟢 |
| s3_uri | 1 | 1 | 100% | 🟢 |
| mac_address | 1 | 1 | 100% | 🟢 |
| docker_ref | 1 | 1 | 100% | 🟢 |
| inchi | 1 | 1 | 100% | 🟢 |
| dmy_short_dot | 1 | 0 | 0% | 🔴 |
| wkt | 1 | 1 | 100% | 🟢 |
| cidr | 1 | 1 | 100% | 🟢 |
| loinc | 1 | 1 | 100% | 🟢 |
| sql_standard | 1 | 1 | 100% | 🟢 |
| pan_india | 1 | 1 | 100% | 🟢 |
| orcid | 1 | 1 | 100% | 🟢 |
| figi | 1 | 1 | 100% | 🟢 |
| binary | 1 | 1 | 100% | 🟢 |
| urn | 1 | 1 | 100% | 🟢 |
| phone_e164 | 1 | 1 | 100% | 🟢 |
| swift_bic | 1 | 1 | 100% | 🟢 |
| phone_number | 1 | 1 | 100% | 🟢 |
| vin | 1 | 1 | 100% | 🟢 |
| data_uri | 1 | 1 | 100% | 🟢 |
| cas_number | 1 | 1 | 100% | 🟢 |
| icd10 | 1 | 1 | 100% | 🟢 |
| mgrs | 1 | 1 | 100% | 🟢 |
| email_display | 1 | 1 | 100% | 🟢 |
| dmy_hm | 1 | 1 | 100% | 🟢 |
| iso | 1 | 1 | 100% | 🟢 |
| geojson | 1 | 1 | 100% | 🟢 |
| iata_code | 1 | 1 | 100% | 🟢 |
| os | 1 | 1 | 100% | 🟢 |
| hm_12h | 1 | 1 | 100% | 🟢 |
| dms | 1 | 1 | 100% | 🟢 |
| hcpcs | 1 | 1 | 100% | 🟢 |
| iso6346 | 1 | 1 | 100% | 🟢 |
| snowflake_id | 1 | 1 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 0 | 0% | 🔴 |
| eu_vat | 1 | 1 | 100% | 🟢 |
| ulid | 1 | 1 | 100% | 🟢 |
| isrc | 1 | 1 | 100% | 🟢 |
| aba_routing | 1 | 1 | 100% | 🟢 |
| region | 1 | 0 | 0% | 🔴 |
| h3 | 1 | 1 | 100% | 🟢 |
| bsb | 1 | 1 | 100% | 🟢 |
| hms_12h | 1 | 1 | 100% | 🟢 |
| color_hsl | 1 | 1 | 100% | 🟢 |

## Actionability Evaluation

Can analysts safely TRY_CAST using FineType's format_string predictions?
**Target:** >95% success rate for datetime types

### By Type

| Type | Columns | Values | Success Rate | Status |
|---|---|---|---|---|
| integer_number | 18 | 49118 | 100% | 🟢 |
| decimal_number | 10 | 14702 | 99.6% | 🟢 |
| iso_8601 | 9 | 710 | 29.6% | 🔴 |
| amount | 9 | 1651 | 100% | 🟢 |
| categorical | 9 | 15661 | 100% | 🟢 |
| country | 8 | 42544 | 100% | 🟢 |
| latitude | 7 | 36212 | 100% | 🟢 |
| longitude | 7 | 36212 | 99.7% | 🟢 |
| postal_code | 6 | 460 | 100% | 🟢 |
| country_code | 6 | 847 | 100% | 🟢 |
| entity_name | 6 | 8018 | 100% | 🟢 |
| url | 6 | 420 | 100% | 🟢 |
| city | 5 | 40986 | 100% | 🟢 |
| full_name | 5 | 1111 | 100% | 🟢 |
| iana | 4 | 29608 | 100% | 🟢 |
| hs_code | 4 | 28291 | 100% | 🟢 |
| ssn | 4 | 380 | 100% | 🟢 |
| dmy_short_dot | 3 | 14232 | 0.7% | 🔴 |
| iso | 3 | 1080 | 100% | 🟢 |
| full_address | 3 | 7858 | 100% | 🟢 |
| continent | 3 | 42396 | 100% | 🟢 |
| email | 3 | 200 | 100% | 🟢 |
| gender | 3 | 1051 | 100% | 🟢 |
| terms | 3 | 260 | 100% | 🟢 |
| ordinal | 3 | 1842 | 100% | 🟢 |
| uuid | 3 | 260 | 100% | 🟢 |
| percentage | 3 | 250 | 100% | 🟢 |
| ip_v4 | 3 | 280 | 100% | 🟢 |
| year | 2 | 14140 | 100% | 🟢 |
| ymd_slash | 2 | 160 | 0% | 🔴 |
| utc | 2 | 7778 | 100% | 🟢 |
| iso_8601_milliseconds | 2 | 28264 | 100% | 🟢 |
| mdy_12h | 2 | 180 | 44.4% | 🔴 |
| amount_minor_int | 2 | 33337 | 100% | 🟢 |
| currency_code | 2 | 200 | 100% | 🟢 |
| geohash | 2 | 14212 | 100% | 🟢 |
| region | 2 | 14183 | 100% | 🟢 |
| unlocode | 2 | 7778 | 100% | 🟢 |
| ean | 2 | 160 | 100% | 🟢 |
| isbn | 2 | 140 | 100% | 🟢 |
| isrc | 2 | 140 | 100% | 🟢 |
| upc | 2 | 140 | 100% | 🟢 |
| abn | 2 | 80 | 100% | 🟢 |
| ein | 2 | 160 | 100% | 🟢 |
| icd10 | 2 | 140 | 100% | 🟢 |
| first_name | 2 | 160 | 100% | 🟢 |
| height | 2 | 0 | 0% | 🔴 |
| last_name | 2 | 160 | 100% | 🟢 |
| weight | 2 | 160 | 100% | 🟢 |
| mime_type | 2 | 180 | 100% | 🟢 |
| smiles | 2 | 130 | 100% | 🟢 |
| locale_code | 2 | 140 | 100% | 🟢 |
| jwt | 2 | 160 | 100% | 🟢 |
| git_sha | 2 | 160 | 100% | 🟢 |
| version | 2 | 300 | 100% | 🟢 |
| snowflake_id | 2 | 80 | 100% | 🟢 |
| query_string | 1 | 100 | 100% | 🟢 |
| day_of_week | 1 | 80 | 100% | 🟢 |
| month_name | 1 | 80 | 100% | 🟢 |
| periodicity | 1 | 100 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 80 | 0% | 🔴 |
| compact_ym | 1 | 80 | 0% | 🔴 |
| full_month_no_comma | 1 | 80 | 0% | 🔴 |
| iso_8601 | 1 | 0 | 0% | 🔴 |
| unix_milliseconds | 1 | 80 | 100% | 🟢 |
| unix_seconds | 1 | 80 | 100% | 🟢 |
| hm_12h | 1 | 80 | 100% | 🟢 |
| hm_24h | 1 | 100 | 100% | 🟢 |
| hms_12h | 1 | 80 | 100% | 🟢 |
| hms_24h | 1 | 80 | 100% | 🟢 |
| dmy_hm | 1 | 80 | 100% | 🟢 |
| iso_8601_millis_offset | 1 | 100 | 0% | 🔴 |
| rfc_2822 | 1 | 80 | 100% | 🟢 |
| sql_standard | 1 | 80 | 100% | 🟢 |
| aba_routing | 1 | 80 | 100% | 🟢 |
| bsb | 1 | 80 | 100% | 🟢 |
| swift_bic | 1 | 80 | 100% | 🟢 |
| amount_nodecimal | 1 | 60 | 66.7% | 🔴 |
| credit_card_number | 1 | 80 | 100% | 🟢 |
| figi | 1 | 80 | 100% | 🟢 |
| isin | 1 | 100 | 100% | 🟢 |
| coordinates | 1 | 0 | 0% | 🔴 |
| dms | 1 | 80 | 100% | 🟢 |
| mgrs | 1 | 80 | 100% | 🟢 |
| plus_code | 1 | 80 | 100% | 🟢 |
| geojson | 1 | 80 | 100% | 🟢 |
| wkt | 1 | 80 | 100% | 🟢 |
| h3 | 1 | 80 | 100% | 🟢 |
| iata_code | 1 | 7698 | 100% | 🟢 |
| icao_code | 1 | 100 | 100% | 🟢 |
| iso6346 | 1 | 80 | 100% | 🟢 |
| orcid | 1 | 80 | 100% | 🟢 |
| eu_vat | 1 | 80 | 100% | 🟢 |
| pan_india | 1 | 80 | 100% | 🟢 |
| vin | 1 | 80 | 100% | 🟢 |
| cpt | 1 | 80 | 100% | 🟢 |
| dea_number | 1 | 249 | 100% | 🟢 |
| hcpcs | 1 | 80 | 100% | 🟢 |
| loinc | 1 | 80 | 100% | 🟢 |
| email_display | 1 | 80 | 100% | 🟢 |
| phone_e164 | 1 | 80 | 100% | 🟢 |
| phone_number | 1 | 60 | 100% | 🟢 |
| binary | 1 | 891 | 100% | 🟢 |
| color_hex | 1 | 80 | 100% | 🟢 |
| color_hsl | 1 | 80 | 100% | 🟢 |
| alphanumeric_id | 1 | 50 | 100% | 🟢 |
| increment | 1 | 891 | 100% | 🟢 |
| numeric_code | 1 | 247 | 100% | 🟢 |
| scientific_notation | 1 | 100 | 0% | 🔴 |
| cas_number | 1 | 80 | 100% | 🟢 |
| inchi | 1 | 80 | 100% | 🟢 |
| protein_sequence | 1 | 100 | 100% | 🟢 |
| paragraph | 1 | 60 | 100% | 🟢 |
| aws_arn | 1 | 80 | 100% | 🟢 |
| s3_uri | 1 | 80 | 100% | 🟢 |
| token_urlsafe | 1 | 80 | 100% | 🟢 |
| docker_ref | 1 | 80 | 100% | 🟢 |
| tsid | 1 | 80 | 100% | 🟢 |
| ulid | 1 | 80 | 100% | 🟢 |
| cidr | 1 | 80 | 100% | 🟢 |
| data_uri | 1 | 80 | 100% | 🟢 |
| http_method | 1 | 100 | 100% | 🟢 |
| mac_address | 1 | 80 | 100% | 🟢 |
| top_level_domain | 1 | 14132 | 100% | 🟢 |
| urn | 1 | 80 | 100% | 🟢 |

### Below Target (<95%)

| Dataset | Column | Type | Format | Success Rate |
|---|---|---|---|---|
| ecommerce_orders | order_date | iso_8601 | `` | 0% 🔴 |
| financial_data | date | iso_8601 | `` | 0% 🔴 |
| datetime_formats | us_date | ymd_slash | `` | 0% 🔴 |
| datetime_formats | eu_date | ymd_slash | `` | 0% 🔴 |
| datetime_formats | year | compact_ym | `` | 0% 🔴 |
| medical_records | visit_date | iso_8601 | `` | 0% 🔴 |
| network_logs | timestamp | iso_8601_millis_offset | `` | 0% 🔴 |
| network_logs | user_agent | mdy_12h | `` | 0% 🔴 |
| multilingual | date | iso_8601 | `` | 0% 🔴 |
| sports_events | event_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | eu_dot_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | abbreviated_month_date | abbrev_month_no_comma | `` | 0% 🔴 |
| datetime_formats_extended | long_full_month_date | full_month_no_comma | `` | 0% 🔴 |
| weather_stations_json | observation_date | iso | `` | 0% 🔴 |
| earthquakes_2024 | horizontalError | dmy_short_dot | `` | 0% 🔴 |
| people_directory | height_cm | height | `` | 0% 🔴 |
| financial_data | market_cap | longitude | `` | 0% 🔴 |
| datetime_formats | duration_iso | iso_8601 | `` | 0% 🔴 |
| geography_data | coordinates | coordinates | `` | 0% 🔴 |
| codes_and_ids | iban | snowflake_id | `` | 0% 🔴 |
| medical_records | blood_pressure | decimal_number | `` | 0% 🔴 |
| medical_records | height_in | height | `` | 0% 🔴 |
| sports_events | event_id | scientific_notation | `` | 0% 🔴 |
| api_users_json | address.city | city | `` | 0% 🔴 |
| api_users_json | address.country | country_code | `` | 0% 🔴 |
| api_users_json | address.postal_code | postal_code | `` | 0% 🔴 |
| api_users_json | email | email | `` | 0% 🔴 |
| api_users_json | name | full_name | `` | 0% 🔴 |
| api_users_json | phone | abn | `` | 0% 🔴 |
| api_users_json | profile_url | url | `` | 0% 🔴 |
| weather_stations_json | humidity_pct | hs_code | `` | 0% 🔴 |
| weather_stations_json | location.city | city | `` | 0% 🔴 |
| weather_stations_json | location.country | country_code | `` | 0% 🔴 |
| weather_stations_json | location.latitude | latitude | `` | 0% 🔴 |
| weather_stations_json | location.longitude | longitude | `` | 0% 🔴 |
| weather_stations_json | precipitation_mm | decimal_number | `` | 0% 🔴 |
| weather_stations_json | station_name | entity_name | `` | 0% 🔴 |
| weather_stations_json | temperature_c | decimal_number | `` | 0% 🔴 |
| codes_and_ids | semantic_version | dmy_short_dot | `` | 28.7% 🔴 |
| multilingual | price | amount_nodecimal | `` | 66.7% 🔴 |
| tech_systems | version | dmy_short_dot | `` | 93.8% 🔴 |

## Evaluation Components

| Component | Scope | Target | Status |
|---|---|---|---|
| Profile regression | 190 columns, 29 datasets | No regressions | 🟡 |
| Precision per type | SOTAB/GitTables | 🟢≥95% per type | Run `make eval-sotab-cli` |
| Overcall analysis | SOTAB/GitTables | <5% FP rate | Run `make eval-sotab-cli` |
| Actionability | Profile eval datetime | >95% parse rate | 🟢 |
| Confidence calibration | SOTAB/GitTables | Gap <10pp | Run `make eval-sotab-cli` |
| Domain accuracy | SOTAB format-detectable | >80% | Run `make eval-sotab-cli` |

---
*Generated by eval-report (NNFT-184, Rust port of eval_report.py)*
