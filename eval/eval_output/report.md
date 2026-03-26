# FineType Evaluation Report

**Generated:** 2026-03-26 19:31

## Headline Metrics

| Metric | Value | Status |
|---|---|---|
| Profile label accuracy | 182/214 (85%) | 🟡 |
| Profile domain accuracy | 197/214 (92.1%) | 🟡 |
| Actionability (datetime) | 541703/543310 (99.7%) | 🟢 |
| Columns with >95% parse rate | 266/310 | |
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

**Label accuracy:** 182/214 (85%)
**Domain accuracy:** 197/214 (92.1%)

### Misclassifications

| Dataset | Column | Predicted | Expected | Confidence |
|---|---|---|---|---|
| ecommerce_orders | phone | ssn | phone_number | 1.00 |
| api_users_json | phone | abn | phone_number | 1.00 |
| codes_and_ids | ean | upc | ean | 1.00 |
| datetime_formats_extended | long_full_month_date | full_month_no_comma | long_full_month | 1.00 |
| finance_coverage | bitcoin_address | full_address | bitcoin_address | 1.00 |
| earthquakes_2024 | depthError | latitude | decimal_number | 1.00 |
| people_directory | phone | ssn | phone_number | 1.00 |
| datetime_formats_extended | abbreviated_month_date | abbrev_month_no_comma | abbreviated_month | 0.99 |
| datetime_formats_extended | eu_dot_date | iso_8601 | dmy_dot | 0.99 |
| tech_systems | user_agent | jwt | user_agent | 0.99 |
| finance_coverage | currency_symbol | currency_code | currency_symbol | 0.99 |
| airports | icao | unlocode | icao_code | 0.99 |
| network_logs | status_code | compact_ym | integer_number | 0.98 |
| earthquakes_2024 | gap | year | decimal_number | 0.98 |
| technology_coverage | ip_v6 | aws_arn | ip_v6 | 0.98 |
| finance_coverage | isin | isrc | isin | 0.97 |
| earthquakes_2024 | depth | longitude | decimal_number | 0.94 |
| medical_records | npi | upc | npi | 0.94 |
| codes_and_ids | issn | ein | issn | 0.91 |
| new_technology | git_sha | tsid | hash | 0.90 |
| new_geography | geojson | wkt | json | 0.88 |
| tech_systems | port | ean | integer_number | 0.88 |
| earthquakes_2024 | id | geohash | alphanumeric_id | 0.87 |
| earthquakes_2024 | place | region | full_address | 0.85 |
| datetime_formats | year | compact_ym | year | 0.81 |
| technology_coverage | ip_v4_with_port | ip_v4 | ip_v4_with_port | 0.70 |
| codes_and_ids | sha256 | ethereum_address | hash | 0.70 |
| books_catalog | author | email_display | full_name | 0.67 |
| datetime_coverage | dmy_dash | mdy_dash | dmy_dash | 0.51 |
| iris | petal_length | cidr | decimal_number | 0.50 |
| scientific_measurements | measurement_unit | categorical | measurement_unit | 0.43 |
| multilingual | locale | scientific_notation | locale_code | 0.23 |

## Precision Per Type (Profile Eval)

| Predicted Type | Predicted | Correct | Precision | Status |
|---|---|---|---|---|
| decimal_number | 18 | 18 | 100% | 🟢 |
| country | 8 | 8 | 100% | 🟢 |
| latitude | 6 | 5 | 83.3% | 🟡 |
| longitude | 6 | 5 | 83.3% | 🟡 |
| state | 5 | 5 | 100% | 🟢 |
| full_name | 5 | 5 | 100% | 🟢 |
| country_code | 5 | 5 | 100% | 🟢 |
| city | 5 | 5 | 100% | 🟢 |
| postal_code | 4 | 4 | 100% | 🟢 |
| entity_name | 4 | 4 | 100% | 🟢 |
| url | 4 | 4 | 100% | 🟢 |
| ip_v4 | 4 | 3 | 75% | 🔴 |
| full_address | 3 | 2 | 66.7% | 🔴 |
| ssn | 3 | 1 | 33.3% | 🔴 |
| upc | 3 | 1 | 33.3% | 🔴 |
| percentage | 3 | 3 | 100% | 🟢 |
| terms | 3 | 3 | 100% | 🟢 |
| gender | 3 | 3 | 100% | 🟢 |
| email | 3 | 3 | 100% | 🟢 |
| uuid | 3 | 3 | 100% | 🟢 |
| geohash | 2 | 1 | 50% | 🔴 |
| isrc | 2 | 1 | 50% | 🔴 |
| clf | 2 | 2 | 100% | 🟢 |
| year | 2 | 1 | 50% | 🔴 |
| email_display | 2 | 1 | 50% | 🔴 |
| jwt | 2 | 1 | 50% | 🔴 |
| ein | 2 | 1 | 50% | 🔴 |
| utc | 2 | 2 | 100% | 🟢 |
| tsid | 2 | 1 | 50% | 🔴 |
| weight | 2 | 2 | 100% | 🟢 |
| cidr | 2 | 1 | 50% | 🔴 |
| wkt | 2 | 1 | 50% | 🔴 |
| first_name | 2 | 2 | 100% | 🟢 |
| mdy_dash | 2 | 1 | 50% | 🔴 |
| iso_8601_milliseconds | 2 | 2 | 100% | 🟢 |
| iana | 2 | 2 | 100% | 🟢 |
| last_name | 2 | 2 | 100% | 🟢 |
| unlocode | 2 | 1 | 50% | 🔴 |
| compact_ym | 2 | 0 | 0% | 🔴 |
| scientific_notation | 2 | 1 | 50% | 🔴 |
| abn | 2 | 1 | 50% | 🔴 |
| height | 2 | 2 | 100% | 🟢 |
| iso_8601 | 2 | 1 | 50% | 🔴 |
| aws_arn | 2 | 1 | 50% | 🔴 |
| categorical | 2 | 1 | 50% | 🔴 |
| binary | 1 | 1 | 100% | 🟢 |
| iso_8601_offset | 1 | 1 | 100% | 🟢 |
| hs_code | 1 | 1 | 100% | 🟢 |
| loinc | 1 | 1 | 100% | 🟢 |
| cas_number | 1 | 1 | 100% | 🟢 |
| lei | 1 | 1 | 100% | 🟢 |
| hostname | 1 | 1 | 100% | 🟢 |
| integer_number | 1 | 1 | 100% | 🟢 |
| mgrs | 1 | 1 | 100% | 🟢 |
| swift_bic | 1 | 1 | 100% | 🟢 |
| smiles | 1 | 1 | 100% | 🟢 |
| iso_space_zulu | 1 | 1 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 0 | 0% | 🔴 |
| h3 | 1 | 1 | 100% | 🟢 |
| calver | 1 | 1 | 100% | 🟢 |
| color_hsl | 1 | 1 | 100% | 🟢 |
| day_of_week | 1 | 1 | 100% | 🟢 |
| orcid | 1 | 1 | 100% | 🟢 |
| ymd_dot | 1 | 1 | 100% | 🟢 |
| mdy_12h | 1 | 1 | 100% | 🟢 |
| iata_code | 1 | 1 | 100% | 🟢 |
| vin | 1 | 1 | 100% | 🟢 |
| ulid | 1 | 1 | 100% | 🟢 |
| iso_week | 1 | 1 | 100% | 🟢 |
| full_month_no_comma | 1 | 0 | 0% | 🔴 |
| urn | 1 | 1 | 100% | 🟢 |
| data_uri | 1 | 1 | 100% | 🟢 |
| decimal_number_comma | 1 | 1 | 100% | 🟢 |
| phone_e164 | 1 | 1 | 100% | 🟢 |
| iso | 1 | 1 | 100% | 🟢 |
| phone_number | 1 | 1 | 100% | 🟢 |
| month_name | 1 | 1 | 100% | 🟢 |
| quarter | 1 | 1 | 100% | 🟢 |
| sql_standard | 1 | 1 | 100% | 🟢 |
| aba_routing | 1 | 1 | 100% | 🟢 |
| mac_address | 1 | 1 | 100% | 🟢 |
| fiscal_year | 1 | 1 | 100% | 🟢 |
| username | 1 | 1 | 100% | 🟢 |
| user_agent | 1 | 1 | 100% | 🟢 |
| compact_ymd | 1 | 1 | 100% | 🟢 |
| ean | 1 | 0 | 0% | 🔴 |
| dms | 1 | 1 | 100% | 🟢 |
| file_size | 1 | 1 | 100% | 🟢 |
| credit_card_number | 1 | 1 | 100% | 🟢 |
| bsb | 1 | 1 | 100% | 🟢 |
| locale_code | 1 | 1 | 100% | 🟢 |
| s3_uri | 1 | 1 | 100% | 🟢 |
| eu_vat | 1 | 1 | 100% | 🟢 |
| iso6346 | 1 | 1 | 100% | 🟢 |
| figi | 1 | 1 | 100% | 🟢 |
| rfc_2822 | 1 | 1 | 100% | 🟢 |
| docker_ref | 1 | 1 | 100% | 🟢 |
| icd10 | 1 | 1 | 100% | 🟢 |
| pan_india | 1 | 1 | 100% | 🟢 |
| plus_code | 1 | 1 | 100% | 🟢 |
| snowflake_id | 1 | 1 | 100% | 🟢 |
| inchi | 1 | 1 | 100% | 🟢 |
| region | 1 | 0 | 0% | 🔴 |
| hcpcs | 1 | 1 | 100% | 🟢 |
| compact_dmy | 1 | 1 | 100% | 🟢 |
| cusip | 1 | 1 | 100% | 🟢 |
| cpt | 1 | 1 | 100% | 🟢 |
| hm_12h | 1 | 1 | 100% | 🟢 |
| currency_code | 1 | 0 | 0% | 🔴 |
| ethereum_address | 1 | 0 | 0% | 🔴 |
| hms_12h | 1 | 1 | 100% | 🟢 |

## Actionability Evaluation

Can analysts safely TRY_CAST using FineType's format_string predictions?
**Target:** >95% success rate for datetime types

### By Type

| Type | Columns | Values | Success Rate | Status |
|---|---|---|---|---|
| decimal_number | 19 | 71296 | 99.9% | 🟢 |
| integer_number | 18 | 48896 | 100% | 🟢 |
| country | 11 | 84940 | 100% | 🟢 |
| iso | 10 | 1660 | 96.4% | 🟢 |
| amount | 9 | 1651 | 100% | 🟢 |
| categorical | 7 | 1429 | 100% | 🟢 |
| latitude | 6 | 36112 | 100% | 🟢 |
| longitude | 6 | 36112 | 100% | 🟢 |
| country_code | 6 | 847 | 100% | 🟢 |
| entity_name | 6 | 8018 | 100% | 🟢 |
| iso_8601 | 5 | 370 | 56.8% | 🔴 |
| postal_code | 5 | 360 | 100% | 🟢 |
| city | 5 | 40986 | 100% | 🟢 |
| full_name | 5 | 8749 | 100% | 🟢 |
| url | 5 | 340 | 100% | 🟢 |
| iana | 4 | 29608 | 100% | 🟢 |
| ssn | 4 | 380 | 100% | 🟢 |
| ip_v4 | 4 | 305 | 100% | 🟢 |
| iso_8601 | 3 | 0 | 0% | 🔴 |
| full_address | 3 | 185 | 100% | 🟢 |
| orcid | 3 | 220 | 100% | 🟢 |
| upc | 3 | 220 | 100% | 🟢 |
| ein | 3 | 260 | 100% | 🟢 |
| email | 3 | 200 | 100% | 🟢 |
| gender | 3 | 1051 | 100% | 🟢 |
| terms | 3 | 260 | 100% | 🟢 |
| ordinal | 3 | 1842 | 100% | 🟢 |
| uuid | 3 | 260 | 100% | 🟢 |
| percentage | 3 | 250 | 100% | 🟢 |
| month_name | 2 | 14212 | 100% | 🟢 |
| year | 2 | 14140 | 100% | 🟢 |
| compact_ym | 2 | 180 | 0% | 🔴 |
| compact_ymd | 2 | 272 | 9.2% | 🔴 |
| dmy_short_dot | 2 | 160 | 61.3% | 🔴 |
| mdy_dash | 2 | 50 | 68% | 🔴 |
| ymd_slash | 2 | 160 | 0% | 🔴 |
| utc | 2 | 7778 | 100% | 🟢 |
| clf | 2 | 50 | 0% | 🔴 |
| iso_8601_milliseconds | 2 | 28264 | 100% | 🟢 |
| aba_routing | 2 | 180 | 100% | 🟢 |
| credit_card_number | 2 | 160 | 100% | 🟢 |
| geohash | 2 | 14212 | 100% | 🟢 |
| wkt | 2 | 160 | 100% | 🟢 |
| region | 2 | 14183 | 100% | 🟢 |
| iso6346 | 2 | 140 | 100% | 🟢 |
| unlocode | 2 | 7778 | 100% | 🟢 |
| isrc | 2 | 105 | 100% | 🟢 |
| abn | 2 | 80 | 100% | 🟢 |
| email_display | 2 | 140 | 100% | 🟢 |
| first_name | 2 | 160 | 100% | 🟢 |
| height | 2 | 0 | 0% | 🔴 |
| last_name | 2 | 160 | 100% | 🟢 |
| weight | 2 | 160 | 100% | 🟢 |
| file_size | 2 | 125 | 100% | 🟢 |
| mime_type | 2 | 180 | 100% | 🟢 |
| color_hex | 2 | 284 | 100% | 🟢 |
| alphanumeric_id | 2 | 100 | 100% | 🟢 |
| increment | 2 | 34128 | 100% | 🟢 |
| scientific_notation | 2 | 85 | 29.4% | 🔴 |
| aws_arn | 2 | 105 | 100% | 🟢 |
| jwt | 2 | 160 | 100% | 🟢 |
| tsid | 2 | 160 | 100% | 🟢 |
| cidr | 2 | 230 | 100% | 🟢 |
| top_level_domain | 2 | 14232 | 100% | 🟢 |
| whitespace_separated | 1 | 60 | 100% | 🟢 |
| query_string | 1 | 100 | 100% | 🟢 |
| day_of_week | 1 | 80 | 100% | 🟢 |
| periodicity | 1 | 100 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 80 | 0% | 🔴 |
| compact_dmy | 1 | 25 | 100% | 🟢 |
| full_month_no_comma | 1 | 80 | 0% | 🔴 |
| iso_week | 1 | 25 | 100% | 🟢 |
| month_year_slash | 1 | 247 | 0% | 🔴 |
| ymd_dot | 1 | 25 | 100% | 🟢 |
| unix_milliseconds | 1 | 80 | 100% | 🟢 |
| unix_seconds | 1 | 80 | 100% | 🟢 |
| fiscal_year | 1 | 25 | 100% | 🟢 |
| quarter | 1 | 25 | 0% | 🔴 |
| hm_12h | 1 | 80 | 100% | 🟢 |
| hm_24h | 1 | 100 | 100% | 🟢 |
| hms_12h | 1 | 80 | 100% | 🟢 |
| hms_24h | 1 | 80 | 100% | 🟢 |
| iso_8601_millis_offset | 1 | 100 | 0% | 🔴 |
| iso_8601_offset | 1 | 25 | 100% | 🟢 |
| iso_space_zulu | 1 | 25 | 100% | 🟢 |
| mdy_12h | 1 | 80 | 100% | 🟢 |
| rfc_2822 | 1 | 80 | 100% | 🟢 |
| sql_standard | 1 | 80 | 100% | 🟢 |
| bsb | 1 | 80 | 100% | 🟢 |
| swift_bic | 1 | 80 | 100% | 🟢 |
| ethereum_address | 1 | 80 | 100% | 🟢 |
| amount_nodecimal | 1 | 60 | 66.7% | 🔴 |
| currency_code | 1 | 25 | 100% | 🟢 |
| cusip | 1 | 25 | 100% | 🟢 |
| figi | 1 | 80 | 100% | 🟢 |
| lei | 1 | 25 | 100% | 🟢 |
| coordinates | 1 | 0 | 0% | 🔴 |
| dms | 1 | 80 | 100% | 🟢 |
| mgrs | 1 | 80 | 100% | 🟢 |
| plus_code | 1 | 80 | 100% | 🟢 |
| h3 | 1 | 80 | 100% | 🟢 |
| hs_code | 1 | 80 | 100% | 🟢 |
| iata_code | 1 | 7698 | 100% | 🟢 |
| icao_code | 1 | 100 | 100% | 🟢 |
| ean | 1 | 80 | 100% | 🟢 |
| eu_vat | 1 | 80 | 100% | 🟢 |
| pan_india | 1 | 80 | 100% | 🟢 |
| vin | 1 | 80 | 100% | 🟢 |
| cpt | 1 | 80 | 100% | 🟢 |
| dea_number | 1 | 249 | 100% | 🟢 |
| hcpcs | 1 | 80 | 100% | 🟢 |
| icd10 | 1 | 80 | 100% | 🟢 |
| loinc | 1 | 80 | 100% | 🟢 |
| phone_e164 | 1 | 80 | 100% | 🟢 |
| phone_number | 1 | 60 | 100% | 🟢 |
| username | 1 | 25 | 100% | 🟢 |
| binary | 1 | 891 | 100% | 🟢 |
| color_hsl | 1 | 80 | 100% | 🟢 |
| decimal_number_comma | 1 | 25 | 100% | 🟢 |
| cas_number | 1 | 80 | 100% | 🟢 |
| inchi | 1 | 80 | 100% | 🟢 |
| protein_sequence | 1 | 100 | 100% | 🟢 |
| smiles | 1 | 80 | 100% | 🟢 |
| s3_uri | 1 | 80 | 100% | 🟢 |
| locale_code | 1 | 80 | 100% | 🟢 |
| token_urlsafe | 1 | 80 | 100% | 🟢 |
| calver | 1 | 25 | 100% | 🟢 |
| docker_ref | 1 | 80 | 100% | 🟢 |
| snowflake_id | 1 | 80 | 100% | 🟢 |
| ulid | 1 | 80 | 100% | 🟢 |
| data_uri | 1 | 80 | 100% | 🟢 |
| hostname | 1 | 80 | 100% | 🟢 |
| http_method | 1 | 100 | 100% | 🟢 |
| mac_address | 1 | 80 | 100% | 🟢 |
| urn | 1 | 80 | 100% | 🟢 |
| user_agent | 1 | 100 | 100% | 🟢 |

### Below Target (<95%)

| Dataset | Column | Type | Format | Success Rate |
|---|---|---|---|---|
| countries | region-code | compact_ymd | `` | 0% 🔴 |
| countries | sub-region-code | month_year_slash | `` | 0% 🔴 |
| datetime_formats | us_date | ymd_slash | `` | 0% 🔴 |
| datetime_formats | eu_date | ymd_slash | `` | 0% 🔴 |
| datetime_formats | year | compact_ym | `` | 0% 🔴 |
| network_logs | timestamp | iso_8601_millis_offset | `` | 0% 🔴 |
| network_logs | status_code | compact_ym | `` | 0% 🔴 |
| multilingual | date | iso | `` | 0% 🔴 |
| datetime_formats_extended | eu_dot_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | abbreviated_month_date | abbrev_month_no_comma | `` | 0% 🔴 |
| datetime_formats_extended | long_full_month_date | full_month_no_comma | `` | 0% 🔴 |
| datetime_formats_extended | european_timestamp | iso_8601 | `` | 0% 🔴 |
| weather_stations_json | observation_date | iso | `` | 0% 🔴 |
| datetime_coverage | clf_timestamp | clf | `` | 0% 🔴 |
| datetime_coverage | syslog_bsd | clf | `` | 0% 🔴 |
| people_directory | height_cm | height | `` | 0% 🔴 |
| datetime_formats | duration_iso | iso_8601 | `` | 0% 🔴 |
| geography_data | coordinates | coordinates | `` | 0% 🔴 |
| medical_records | patient_id | iso_8601 | `` | 0% 🔴 |
| medical_records | blood_pressure | decimal_number | `` | 0% 🔴 |
| medical_records | height_in | height | `` | 0% 🔴 |
| multilingual | locale | scientific_notation | `` | 0% 🔴 |
| sports_events | event_id | iso_8601 | `` | 0% 🔴 |
| api_users_json | address.city | city | `` | 0% 🔴 |
| api_users_json | address.country | country_code | `` | 0% 🔴 |
| api_users_json | address.postal_code | postal_code | `` | 0% 🔴 |
| api_users_json | email | email | `` | 0% 🔴 |
| api_users_json | name | full_name | `` | 0% 🔴 |
| api_users_json | phone | abn | `` | 0% 🔴 |
| api_users_json | profile_url | url | `` | 0% 🔴 |
| weather_stations_json | humidity_pct | decimal_number | `` | 0% 🔴 |
| weather_stations_json | location.city | city | `` | 0% 🔴 |
| weather_stations_json | location.country | country_code | `` | 0% 🔴 |
| weather_stations_json | location.latitude | latitude | `` | 0% 🔴 |
| weather_stations_json | location.longitude | longitude | `` | 0% 🔴 |
| weather_stations_json | precipitation_mm | decimal_number | `` | 0% 🔴 |
| weather_stations_json | station_name | entity_name | `` | 0% 🔴 |
| weather_stations_json | temperature_c | decimal_number | `` | 0% 🔴 |
| weather_stations_json | wind_speed_kmh | decimal_number | `` | 0% 🔴 |
| datetime_coverage | quarter | quarter | `` | 0% 🔴 |
| codes_and_ids | semantic_version | dmy_short_dot | `` | 28.7% 🔴 |
| datetime_coverage | dmy_dash | mdy_dash | `` | 36% 🔴 |
| multilingual | price | amount_nodecimal | `` | 66.7% 🔴 |
| tech_systems | version | dmy_short_dot | `` | 93.8% 🔴 |

## Evaluation Components

| Component | Scope | Target | Status |
|---|---|---|---|
| Profile regression | 214 columns, 33 datasets | No regressions | 🟡 |
| Precision per type | SOTAB/GitTables | 🟢≥95% per type | Run `make eval-sotab-cli` |
| Overcall analysis | SOTAB/GitTables | <5% FP rate | Run `make eval-sotab-cli` |
| Actionability | Profile eval datetime | >95% parse rate | 🟢 |
| Confidence calibration | SOTAB/GitTables | Gap <10pp | Run `make eval-sotab-cli` |
| Domain accuracy | SOTAB format-detectable | >80% | Run `make eval-sotab-cli` |

---
*Generated by eval-report (NNFT-184, Rust port of eval_report.py)*
