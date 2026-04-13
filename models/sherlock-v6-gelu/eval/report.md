# FineType Evaluation Report

**Generated:** 2026-04-11 12:17

## Headline Metrics

| Metric | Value | Status |
|---|---|---|
| Profile label accuracy | 167/214 (78%) | 🔴 |
| Profile domain accuracy | 187/214 (87.4%) | 🔴 |
| Actionability (datetime) | 513301/542956 (94.5%) | 🟡 |
| Columns with >95% parse rate | 258/306 | |
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

**Label accuracy:** 167/214 (78%)
**Domain accuracy:** 187/214 (87.4%)

### Misclassifications

| Dataset | Column | Predicted | Expected | Confidence |
|---|---|---|---|---|
| medical_records | first_name | email_display | first_name | 1.00 |
| representation_coverage | scientific_notation | decimal_number | scientific_notation | 1.00 |
| people_directory | full_name | email_display | full_name | 1.00 |
| finance_coverage | bitcoin_address | full_address | bitcoin_address | 1.00 |
| books_catalog | author | email_display | full_name | 1.00 |
| datetime_formats | year | compact_ym | year | 1.00 |
| airports | name | full_address | full_name | 1.00 |
| scientific_measurements | ph_value | dmy_short_dot | decimal_number | 1.00 |
| airports | icao | unlocode | icao_code | 1.00 |
| weather_stations_json | precipitation_mm | dmy_short_dot | decimal_number | 1.00 |
| api_users_json | phone | abn | phone_number | 1.00 |
| earthquakes_2024 | magError | dmy_short_dot | decimal_number | 0.99 |
| new_geography | geojson | wkt | json | 0.99 |
| ecommerce_orders | shipping_country | iana | country | 0.99 |
| datetime_formats_extended | eu_dot_date | iso_8601 | dmy_dot | 0.99 |
| ecommerce_orders | phone | ssn | phone_number | 0.98 |
| earthquakes_2024 | id | geohash | alphanumeric_id | 0.98 |
| people_directory | phone | ssn | phone_number | 0.98 |
| finance_coverage | currency_symbol | locale_code | currency_symbol | 0.97 |
| iris | sepal_length | scientific_notation | decimal_number | 0.95 |
| api_users_json | name | first_name | full_name | 0.95 |
| new_technology | git_sha | ethereum_address | hash | 0.91 |
| technology_coverage | ip_v4_with_port | cidr | ip_v4_with_port | 0.90 |
| finance_coverage | cusip | eu_vat | cusip | 0.89 |
| codes_and_ids | sha256 | ethereum_address | hash | 0.88 |
| earthquakes_2024 | rms | dmy_short_dot | decimal_number | 0.82 |
| finance_coverage | isin | eu_vat | isin | 0.80 |
| network_logs | user_agent | docker_ref | user_agent | 0.75 |
| earthquakes_2024 | depthError | ip_v4 | decimal_number | 0.75 |
| scientific_measurements | measurement_unit | iata_code | measurement_unit | 0.75 |
| weather_stations_json | wind_speed_kmh | ip_v4 | decimal_number | 0.73 |
| iris | petal_length | scientific_notation | decimal_number | 0.73 |
| weather_stations_json | observation_date | iso_8601 | iso | 0.70 |
| new_identity | upc | ean | upc | 0.70 |
| iris | petal_width | yield | decimal_number | 0.68 |
| earthquakes_2024 | dmin | ip_v4 | decimal_number | 0.66 |
| technology_coverage | ip_v6 | hash | ip_v6 | 0.55 |
| datetime_coverage | ymd_dot | dmy_dot | ymd_dot | 0.55 |
| network_logs | status_code | compact_ym | integer_number | 0.54 |
| datetime_formats_extended | abbreviated_month_date | abbrev_month_no_comma | abbreviated_month | 0.53 |
| datetime_formats_extended | long_full_month_date | full_month_no_comma | long_full_month | 0.51 |
| datetime_coverage | dmy_dash | mdy_dash | dmy_dash | 0.49 |
| earthquakes_2024 | gap | icd10 | decimal_number | 0.41 |
| datetime_coverage | compact_ymd | compact_dmy | compact_ymd | 0.38 |
| multilingual | locale | categorical | locale_code | 0.33 |
| earthquakes_2024 | place | city | full_address | 0.31 |
| tech_systems | user_agent | color_hsl | user_agent | 0.18 |

## Precision Per Type (Profile Eval)

| Predicted Type | Predicted | Correct | Precision | Status |
|---|---|---|---|---|
| decimal_number | 12 | 11 | 91.7% | 🟡 |
| city | 7 | 6 | 85.7% | 🟡 |
| ip_v4 | 6 | 3 | 50% | 🔴 |
| country | 6 | 6 | 100% | 🟢 |
| state | 5 | 5 | 100% | 🟢 |
| longitude | 5 | 5 | 100% | 🟢 |
| latitude | 5 | 5 | 100% | 🟢 |
| country_code | 5 | 5 | 100% | 🟢 |
| postal_code | 4 | 4 | 100% | 🟢 |
| full_address | 4 | 2 | 50% | 🔴 |
| url | 4 | 4 | 100% | 🟢 |
| dmy_short_dot | 4 | 0 | 0% | 🔴 |
| entity_name | 4 | 4 | 100% | 🟢 |
| email_display | 4 | 1 | 25% | 🔴 |
| gender | 3 | 3 | 100% | 🟢 |
| percentage | 3 | 3 | 100% | 🟢 |
| email | 3 | 3 | 100% | 🟢 |
| uuid | 3 | 3 | 100% | 🟢 |
| iso_8601 | 3 | 1 | 33.3% | 🔴 |
| iana | 3 | 2 | 66.7% | 🔴 |
| ssn | 3 | 1 | 33.3% | 🔴 |
| terms | 3 | 3 | 100% | 🟢 |
| eu_vat | 3 | 1 | 33.3% | 🔴 |
| utc | 2 | 2 | 100% | 🟢 |
| ean | 2 | 1 | 50% | 🔴 |
| compact_dmy | 2 | 1 | 50% | 🔴 |
| last_name | 2 | 2 | 100% | 🟢 |
| docker_ref | 2 | 1 | 50% | 🔴 |
| height | 2 | 2 | 100% | 🟢 |
| iata_code | 2 | 1 | 50% | 🔴 |
| categorical | 2 | 1 | 50% | 🔴 |
| wkt | 2 | 1 | 50% | 🔴 |
| ethereum_address | 2 | 0 | 0% | 🔴 |
| abn | 2 | 1 | 50% | 🔴 |
| mdy_dash | 2 | 1 | 50% | 🔴 |
| geohash | 2 | 1 | 50% | 🔴 |
| icd10 | 2 | 1 | 50% | 🔴 |
| scientific_notation | 2 | 0 | 0% | 🔴 |
| first_name | 2 | 1 | 50% | 🔴 |
| color_hsl | 2 | 1 | 50% | 🔴 |
| locale_code | 2 | 1 | 50% | 🔴 |
| weight | 2 | 2 | 100% | 🟢 |
| cidr | 2 | 1 | 50% | 🔴 |
| iso_8601_milliseconds | 2 | 2 | 100% | 🟢 |
| integer_number | 2 | 2 | 100% | 🟢 |
| compact_ym | 2 | 0 | 0% | 🔴 |
| unlocode | 2 | 1 | 50% | 🔴 |
| ulid | 1 | 1 | 100% | 🟢 |
| tsid | 1 | 1 | 100% | 🟢 |
| snowflake_id | 1 | 1 | 100% | 🟢 |
| sql_standard | 1 | 1 | 100% | 🟢 |
| pan_india | 1 | 1 | 100% | 🟢 |
| region | 1 | 1 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 0 | 0% | 🔴 |
| jwt | 1 | 1 | 100% | 🟢 |
| iso_8601_offset | 1 | 1 | 100% | 🟢 |
| rfc_2822 | 1 | 1 | 100% | 🟢 |
| ein | 1 | 1 | 100% | 🟢 |
| cpt | 1 | 1 | 100% | 🟢 |
| clf | 1 | 1 | 100% | 🟢 |
| iso6346 | 1 | 1 | 100% | 🟢 |
| npi | 1 | 1 | 100% | 🟢 |
| file_size | 1 | 1 | 100% | 🟢 |
| hm_12h | 1 | 1 | 100% | 🟢 |
| mdy_12h | 1 | 1 | 100% | 🟢 |
| yield | 1 | 0 | 0% | 🔴 |
| h3 | 1 | 1 | 100% | 🟢 |
| plus_code | 1 | 1 | 100% | 🟢 |
| issn | 1 | 1 | 100% | 🟢 |
| hs_code | 1 | 1 | 100% | 🟢 |
| vin | 1 | 1 | 100% | 🟢 |
| inchi | 1 | 1 | 100% | 🟢 |
| rfc_3339 | 1 | 1 | 100% | 🟢 |
| dmy_dot | 1 | 0 | 0% | 🔴 |
| bsb | 1 | 1 | 100% | 🟢 |
| phone_e164 | 1 | 1 | 100% | 🟢 |
| isrc | 1 | 1 | 100% | 🟢 |
| iso_week | 1 | 1 | 100% | 🟢 |
| username | 1 | 1 | 100% | 🟢 |
| syslog_bsd | 1 | 1 | 100% | 🟢 |
| s3_uri | 1 | 1 | 100% | 🟢 |
| hash | 1 | 0 | 0% | 🔴 |
| aws_arn | 1 | 1 | 100% | 🟢 |
| full_name | 1 | 1 | 100% | 🟢 |
| month_name | 1 | 1 | 100% | 🟢 |
| credit_card_number | 1 | 1 | 100% | 🟢 |
| mac_address | 1 | 1 | 100% | 🟢 |
| full_month_no_comma | 1 | 0 | 0% | 🔴 |
| phone_number | 1 | 1 | 100% | 🟢 |
| decimal_number_comma | 1 | 1 | 100% | 🟢 |
| binary | 1 | 1 | 100% | 🟢 |
| calver | 1 | 1 | 100% | 🟢 |
| data_uri | 1 | 1 | 100% | 🟢 |
| loinc | 1 | 1 | 100% | 🟢 |
| fiscal_year | 1 | 1 | 100% | 🟢 |
| aba_routing | 1 | 1 | 100% | 🟢 |
| lei | 1 | 1 | 100% | 🟢 |
| swift_bic | 1 | 1 | 100% | 🟢 |
| dms | 1 | 1 | 100% | 🟢 |
| figi | 1 | 1 | 100% | 🟢 |
| quarter | 1 | 1 | 100% | 🟢 |
| cas_number | 1 | 1 | 100% | 🟢 |
| urn | 1 | 1 | 100% | 🟢 |
| orcid | 1 | 1 | 100% | 🟢 |
| smiles | 1 | 1 | 100% | 🟢 |
| year | 1 | 1 | 100% | 🟢 |
| mgrs | 1 | 1 | 100% | 🟢 |
| hostname | 1 | 1 | 100% | 🟢 |
| day_of_week | 1 | 1 | 100% | 🟢 |
| hms_12h | 1 | 1 | 100% | 🟢 |
| hcpcs | 1 | 1 | 100% | 🟢 |

## Actionability Evaluation

Can analysts safely TRY_CAST using FineType's format_string predictions?
**Target:** >95% success rate for datetime types

### By Type

| Type | Columns | Values | Success Rate | Status |
|---|---|---|---|---|
| integer_number | 20 | 49867 | 100% | 🟢 |
| decimal_number | 13 | 42881 | 99.9% | 🟢 |
| iso_8601 | 10 | 720 | 36.1% | 🔴 |
| amount | 10 | 1711 | 100% | 🟢 |
| country_code | 9 | 43243 | 100% | 🟢 |
| ssn | 8 | 720 | 100% | 🟢 |
| categorical | 8 | 1539 | 100% | 🟢 |
| city | 7 | 56009 | 100% | 🟢 |
| ip_v4 | 7 | 28592 | 100% | 🟢 |
| dmy_short_dot | 6 | 28352 | 0.3% | 🔴 |
| iso | 6 | 1440 | 100% | 🟢 |
| country | 6 | 42195 | 100% | 🟢 |
| url | 6 | 440 | 100% | 🟢 |
| postal_code | 5 | 360 | 100% | 🟢 |
| latitude | 5 | 21980 | 100% | 🟢 |
| longitude | 5 | 21980 | 100% | 🟢 |
| entity_name | 5 | 7958 | 100% | 🟢 |
| full_address | 4 | 7883 | 100% | 🟢 |
| email_display | 4 | 300 | 100% | 🟢 |
| gender | 4 | 15183 | 100% | 🟢 |
| periodicity | 3 | 7898 | 100% | 🟢 |
| compact_dmy | 3 | 297 | 12.5% | 🔴 |
| iana | 3 | 7878 | 100% | 🟢 |
| iata_code | 3 | 7799 | 100% | 🟢 |
| eu_vat | 3 | 130 | 100% | 🟢 |
| email | 3 | 200 | 100% | 🟢 |
| terms | 3 | 260 | 100% | 🟢 |
| uuid | 3 | 260 | 100% | 🟢 |
| percentage | 3 | 250 | 100% | 🟢 |
| whitespace_separated | 2 | 120 | 100% | 🟢 |
| compact_ym | 2 | 180 | 0% | 🔴 |
| mdy_dash | 2 | 50 | 68% | 🔴 |
| utc | 2 | 7778 | 100% | 🟢 |
| iso_8601_milliseconds | 2 | 28264 | 100% | 🟢 |
| iso_8601_offset | 2 | 75 | 33.3% | 🔴 |
| aba_routing | 2 | 33317 | 100% | 🟢 |
| swift_bic | 2 | 130 | 100% | 🟢 |
| ethereum_address | 2 | 160 | 100% | 🟢 |
| geohash | 2 | 14212 | 100% | 🟢 |
| wkt | 2 | 160 | 100% | 🟢 |
| unlocode | 2 | 7778 | 100% | 🟢 |
| ean | 2 | 160 | 100% | 🟢 |
| isrc | 2 | 329 | 100% | 🟢 |
| issn | 2 | 130 | 100% | 🟢 |
| abn | 2 | 80 | 100% | 🟢 |
| hcpcs | 2 | 140 | 100% | 🟢 |
| icd10 | 2 | 14160 | 100% | 🟢 |
| first_name | 2 | 100 | 100% | 🟢 |
| height | 2 | 0 | 0% | 🔴 |
| last_name | 2 | 160 | 100% | 🟢 |
| weight | 2 | 160 | 100% | 🟢 |
| ordinal | 2 | 951 | 100% | 🟢 |
| color_hsl | 2 | 160 | 100% | 🟢 |
| increment | 2 | 991 | 100% | 🟢 |
| scientific_notation | 2 | 300 | 100% | 🟢 |
| locale_code | 2 | 105 | 100% | 🟢 |
| docker_ref | 2 | 180 | 100% | 🟢 |
| snowflake_id | 2 | 80 | 100% | 🟢 |
| cidr | 2 | 105 | 100% | 🟢 |
| query_string | 1 | 100 | 100% | 🟢 |
| day_of_week | 1 | 80 | 100% | 🟢 |
| month_name | 1 | 80 | 100% | 🟢 |
| year | 1 | 60 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 80 | 0% | 🔴 |
| dmy_dot | 1 | 25 | 0% | 🔴 |
| full_month_no_comma | 1 | 80 | 0% | 🔴 |
| iso_week | 1 | 25 | 100% | 🟢 |
| unix_seconds | 1 | 80 | 100% | 🟢 |
| fiscal_year | 1 | 25 | 100% | 🟢 |
| quarter | 1 | 25 | 0% | 🔴 |
| hm_12h | 1 | 80 | 100% | 🟢 |
| hm_24h | 1 | 100 | 100% | 🟢 |
| hms_12h | 1 | 80 | 100% | 🟢 |
| hms_24h | 1 | 80 | 100% | 🟢 |
| clf | 1 | 25 | 0% | 🔴 |
| iso_8601_compact | 1 | 80 | 0% | 🔴 |
| iso_microseconds | 1 | 60 | 0% | 🔴 |
| mdy_12h | 1 | 80 | 100% | 🟢 |
| rfc_2822 | 1 | 80 | 100% | 🟢 |
| rfc_3339 | 1 | 0 | 0% | 🔴 |
| sql_standard | 1 | 80 | 100% | 🟢 |
| syslog_bsd | 1 | 0 | 0% | 🔴 |
| bsb | 1 | 80 | 100% | 🟢 |
| credit_card_number | 1 | 80 | 100% | 🟢 |
| yield | 1 | 150 | 100% | 🟢 |
| figi | 1 | 80 | 100% | 🟢 |
| lei | 1 | 25 | 100% | 🟢 |
| coordinates | 1 | 0 | 0% | 🔴 |
| dms | 1 | 80 | 100% | 🟢 |
| mgrs | 1 | 80 | 100% | 🟢 |
| plus_code | 1 | 80 | 100% | 🟢 |
| h3 | 1 | 80 | 100% | 🟢 |
| region | 1 | 249 | 100% | 🟢 |
| hs_code | 1 | 80 | 100% | 🟢 |
| iso6346 | 1 | 80 | 100% | 🟢 |
| orcid | 1 | 80 | 100% | 🟢 |
| ein | 1 | 80 | 100% | 🟢 |
| pan_india | 1 | 80 | 100% | 🟢 |
| vin | 1 | 80 | 100% | 🟢 |
| cpt | 1 | 80 | 100% | 🟢 |
| loinc | 1 | 80 | 100% | 🟢 |
| npi | 1 | 60 | 100% | 🟢 |
| full_name | 1 | 60 | 100% | 🟢 |
| phone_e164 | 1 | 80 | 100% | 🟢 |
| phone_number | 1 | 60 | 100% | 🟢 |
| username | 1 | 25 | 100% | 🟢 |
| binary | 1 | 891 | 100% | 🟢 |
| file_size | 1 | 25 | 100% | 🟢 |
| mime_type | 1 | 80 | 100% | 🟢 |
| color_hex | 1 | 80 | 100% | 🟢 |
| alphanumeric_id | 1 | 14132 | 100% | 🟢 |
| numeric_code | 1 | 247 | 100% | 🟢 |
| decimal_number_comma | 1 | 25 | 100% | 🟢 |
| cas_number | 1 | 80 | 100% | 🟢 |
| inchi | 1 | 80 | 100% | 🟢 |
| smiles | 1 | 80 | 100% | 🟢 |
| word | 1 | 14132 | 100% | 🟢 |
| aws_arn | 1 | 80 | 100% | 🟢 |
| s3_uri | 1 | 80 | 100% | 🟢 |
| hash | 1 | 25 | 100% | 🟢 |
| jwt | 1 | 80 | 100% | 🟢 |
| token_urlsafe | 1 | 80 | 100% | 🟢 |
| calver | 1 | 25 | 100% | 🟢 |
| tsid | 1 | 80 | 100% | 🟢 |
| ulid | 1 | 80 | 100% | 🟢 |
| data_uri | 1 | 80 | 100% | 🟢 |
| hostname | 1 | 80 | 100% | 🟢 |
| http_method | 1 | 100 | 100% | 🟢 |
| mac_address | 1 | 80 | 100% | 🟢 |
| urn | 1 | 80 | 100% | 🟢 |

### Below Target (<95%)

| Dataset | Column | Type | Format | Success Rate |
|---|---|---|---|---|
| countries | region-code | compact_dmy | `` | 0% 🔴 |
| datetime_formats | us_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats | eu_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats | unix_ms | iso_8601_compact | `` | 0% 🔴 |
| datetime_formats | year | compact_ym | `` | 0% 🔴 |
| datetime_formats | duration_iso | iso_8601 | `` | 0% 🔴 |
| medical_records | patient_id | iso_microseconds | `` | 0% 🔴 |
| network_logs | status_code | compact_ym | `` | 0% 🔴 |
| scientific_measurements | ph_value | dmy_short_dot | `` | 0% 🔴 |
| scientific_measurements | timestamp | iso_8601_offset | `` | 0% 🔴 |
| multilingual | date | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | eu_dot_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | abbreviated_month_date | abbrev_month_no_comma | `` | 0% 🔴 |
| datetime_formats_extended | long_full_month_date | full_month_no_comma | `` | 0% 🔴 |
| datetime_formats_extended | european_timestamp | iso_8601 | `` | 0% 🔴 |
| weather_stations_json | observation_date | iso_8601 | `` | 0% 🔴 |
| weather_stations_json | precipitation_mm | dmy_short_dot | `` | 0% 🔴 |
| earthquakes_2024 | rms | dmy_short_dot | `` | 0% 🔴 |
| earthquakes_2024 | magError | dmy_short_dot | `` | 0% 🔴 |
| datetime_coverage | rfc_3339 | rfc_3339 | `` | 0% 🔴 |
| datetime_coverage | ymd_dot | dmy_dot | `` | 0% 🔴 |
| datetime_coverage | clf_timestamp | clf | `` | 0% 🔴 |
| datetime_coverage | syslog_bsd | syslog_bsd | `` | 0% 🔴 |
| people_directory | height_cm | height | `` | 0% 🔴 |
| geography_data | coordinates | coordinates | `` | 0% 🔴 |
| codes_and_ids | iban | snowflake_id | `` | 0% 🔴 |
| medical_records | blood_pressure | decimal_number | `` | 0% 🔴 |
| medical_records | height_in | height | `` | 0% 🔴 |
| api_users_json | address.city | city | `` | 0% 🔴 |
| api_users_json | address.country | country_code | `` | 0% 🔴 |
| api_users_json | address.postal_code | postal_code | `` | 0% 🔴 |
| api_users_json | email | email | `` | 0% 🔴 |
| api_users_json | name | first_name | `` | 0% 🔴 |
| api_users_json | phone | abn | `` | 0% 🔴 |
| api_users_json | profile_url | url | `` | 0% 🔴 |
| weather_stations_json | humidity_pct | decimal_number | `` | 0% 🔴 |
| weather_stations_json | location.city | city | `` | 0% 🔴 |
| weather_stations_json | location.country | country_code | `` | 0% 🔴 |
| weather_stations_json | location.latitude | latitude | `` | 0% 🔴 |
| weather_stations_json | location.longitude | longitude | `` | 0% 🔴 |
| weather_stations_json | station_name | entity_name | `` | 0% 🔴 |
| weather_stations_json | temperature_c | decimal_number | `` | 0% 🔴 |
| weather_stations_json | wind_speed_kmh | ip_v4 | `` | 0% 🔴 |
| datetime_coverage | quarter | quarter | `` | 0% 🔴 |
| codes_and_ids | semantic_version | dmy_short_dot | `` | 28.7% 🔴 |
| datetime_coverage | dmy_dash | mdy_dash | `` | 36% 🔴 |
| datetime_coverage | compact_ymd | compact_dmy | `` | 48% 🔴 |
| tech_systems | version | dmy_short_dot | `` | 93.8% 🔴 |

## Evaluation Components

| Component | Scope | Target | Status |
|---|---|---|---|
| Profile regression | 214 columns, 33 datasets | No regressions | 🔴 |
| Precision per type | SOTAB/GitTables | 🟢≥95% per type | Run `make eval-sotab-cli` |
| Overcall analysis | SOTAB/GitTables | <5% FP rate | Run `make eval-sotab-cli` |
| Actionability | Profile eval datetime | >95% parse rate | 🟡 |
| Confidence calibration | SOTAB/GitTables | Gap <10pp | Run `make eval-sotab-cli` |
| Domain accuracy | SOTAB format-detectable | >80% | Run `make eval-sotab-cli` |

---
*Generated by eval-report (NNFT-184, Rust port of eval_report.py)*
