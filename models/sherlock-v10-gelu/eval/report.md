# FineType Evaluation Report

**Generated:** 2026-04-12 07:58

## Headline Metrics

| Metric | Value | Status |
|---|---|---|
| Profile label accuracy | 188/227 (82.8%) | 🟡 |
| Profile domain accuracy | 203/227 (89.4%) | 🔴 |
| Actionability (datetime) | 529207/544646 (97.2%) | 🟢 |
| Columns with >95% parse rate | 296/328 | |
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

**Label accuracy:** 188/227 (82.8%)
**Domain accuracy:** 203/227 (89.4%)

### Misclassifications

| Dataset | Column | Predicted | Expected | Confidence |
|---|---|---|---|---|
| people_directory | full_name | email_display | full_name | 1.00 |
| books_catalog | author | email_display | full_name | 1.00 |
| finance_coverage | bitcoin_address | full_address | bitcoin_address | 1.00 |
| airports | icao | unlocode | icao_code | 1.00 |
| datetime_formats | year | compact_ym | year | 1.00 |
| medical_records | first_name | email_display | first_name | 1.00 |
| network_logs | user_agent | docker_ref | user_agent | 1.00 |
| earthquakes_2024 | magError | dmy_short_dot | decimal_number | 1.00 |
| scientific_measurements | ph_value | dmy_short_dot | decimal_number | 1.00 |
| representation_coverage | scientific_notation | decimal_number | scientific_notation | 0.99 |
| api_users_json | phone | abn | phone_number | 0.99 |
| weather_stations_json | precipitation_mm | dmy_short_dot | decimal_number | 0.99 |
| ecommerce_orders | phone | ssn | phone_number | 0.99 |
| people_directory | phone | ssn | phone_number | 0.99 |
| new_geography | geojson | wkt | json | 0.98 |
| new_technology | git_sha | ethereum_address | hash | 0.96 |
| earthquakes_2024 | depthError | ip_v4 | decimal_number | 0.93 |
| codes_and_ids | sha256 | ethereum_address | hash | 0.93 |
| codes_and_ids | swift_code | isin | swift_bic | 0.93 |
| earthquakes_2024 | id | geohash | alphanumeric_id | 0.92 |
| technology_coverage | ip_v4_with_port | cidr | ip_v4_with_port | 0.85 |
| finance_coverage | isin | iban | isin | 0.78 |
| server_logs_json | user_agent | json_array | user_agent | 0.78 |
| technology_coverage | ip_v6 | hash | ip_v6 | 0.76 |
| datetime_formats_extended | eu_dot_date | iso_8601 | dmy_dot | 0.74 |
| ecommerce_orders | shipping_country | iana | country | 0.72 |
| tech_systems | server_hostname | s3_uri | hostname | 0.65 |
| finance_coverage | cusip | eu_vat | cusip | 0.62 |
| weather_stations_json | wind_speed_kmh | ip_v4 | decimal_number | 0.59 |
| tech_systems | user_agent | jwt | user_agent | 0.56 |
| server_logs_json | status_code | postal_code | integer_number | 0.55 |
| multilingual | name | categorical | full_name | 0.53 |
| finance_coverage | currency_symbol | locale_code | currency_symbol | 0.50 |
| iris | petal_length | scientific_notation | decimal_number | 0.48 |
| network_logs | status_code | bsb | integer_number | 0.48 |
| tech_systems | port | aba_routing | integer_number | 0.48 |
| scientific_measurements | measurement_unit | country_code | measurement_unit | 0.43 |
| multilingual | locale | extension | locale_code | 0.41 |
| datetime_coverage | compact_ymd | compact_mdy | compact_ymd | 0.35 |

## Precision Per Type (Profile Eval)

| Predicted Type | Predicted | Correct | Precision | Status |
|---|---|---|---|---|
| decimal_number | 18 | 17 | 94.4% | 🟡 |
| iso_8601 | 6 | 5 | 83.3% | 🟡 |
| country_code | 6 | 5 | 83.3% | 🟡 |
| city | 6 | 6 | 100% | 🟢 |
| country | 6 | 6 | 100% | 🟢 |
| ip_v4 | 6 | 4 | 66.7% | 🔴 |
| full_address | 5 | 4 | 80% | 🟡 |
| url | 5 | 5 | 100% | 🟢 |
| postal_code | 5 | 4 | 80% | 🟡 |
| state | 5 | 5 | 100% | 🟢 |
| longitude | 5 | 5 | 100% | 🟢 |
| latitude | 5 | 5 | 100% | 🟢 |
| email | 4 | 4 | 100% | 🟢 |
| email_display | 4 | 1 | 25% | 🔴 |
| entity_name | 4 | 4 | 100% | 🟢 |
| terms | 4 | 4 | 100% | 🟢 |
| ssn | 3 | 1 | 33.3% | 🔴 |
| uuid | 3 | 3 | 100% | 🟢 |
| iana | 3 | 2 | 66.7% | 🔴 |
| gender | 3 | 3 | 100% | 🟢 |
| percentage | 3 | 3 | 100% | 🟢 |
| dmy_short_dot | 3 | 0 | 0% | 🔴 |
| geohash | 2 | 1 | 50% | 🔴 |
| integer_number | 2 | 2 | 100% | 🟢 |
| ean | 2 | 2 | 100% | 🟢 |
| mdy_dash | 2 | 2 | 100% | 🟢 |
| categorical | 2 | 1 | 50% | 🔴 |
| weight | 2 | 2 | 100% | 🟢 |
| abn | 2 | 1 | 50% | 🔴 |
| unlocode | 2 | 1 | 50% | 🔴 |
| wkt | 2 | 1 | 50% | 🔴 |
| bsb | 2 | 1 | 50% | 🔴 |
| height | 2 | 2 | 100% | 🟢 |
| jwt | 2 | 1 | 50% | 🔴 |
| locale_code | 2 | 1 | 50% | 🔴 |
| last_name | 2 | 2 | 100% | 🟢 |
| utc | 2 | 2 | 100% | 🟢 |
| docker_ref | 2 | 1 | 50% | 🔴 |
| cidr | 2 | 1 | 50% | 🔴 |
| s3_uri | 2 | 1 | 50% | 🔴 |
| ethereum_address | 2 | 0 | 0% | 🔴 |
| aba_routing | 2 | 1 | 50% | 🔴 |
| eu_vat | 2 | 1 | 50% | 🔴 |
| full_month_no_comma | 1 | 1 | 100% | 🟢 |
| hcpcs | 1 | 1 | 100% | 🟢 |
| full_name | 1 | 1 | 100% | 🟢 |
| credit_card_number | 1 | 1 | 100% | 🟢 |
| fiscal_year | 1 | 1 | 100% | 🟢 |
| iso_week | 1 | 1 | 100% | 🟢 |
| hms_12h | 1 | 1 | 100% | 🟢 |
| cpt | 1 | 1 | 100% | 🟢 |
| issn | 1 | 1 | 100% | 🟢 |
| iata_code | 1 | 1 | 100% | 🟢 |
| currency_code | 1 | 1 | 100% | 🟢 |
| compact_mdy | 1 | 0 | 0% | 🔴 |
| calver | 1 | 1 | 100% | 🟢 |
| file_size | 1 | 1 | 100% | 🟢 |
| npi | 1 | 1 | 100% | 🟢 |
| icd10 | 1 | 1 | 100% | 🟢 |
| quarter | 1 | 1 | 100% | 🟢 |
| mac_address | 1 | 1 | 100% | 🟢 |
| figi | 1 | 1 | 100% | 🟢 |
| json_array | 1 | 0 | 0% | 🔴 |
| color_hsl | 1 | 1 | 100% | 🟢 |
| urn | 1 | 1 | 100% | 🟢 |
| iban | 1 | 0 | 0% | 🔴 |
| first_name | 1 | 1 | 100% | 🟢 |
| binary | 1 | 1 | 100% | 🟢 |
| orcid | 1 | 1 | 100% | 🟢 |
| iso_8601_offset | 1 | 1 | 100% | 🟢 |
| syslog_bsd | 1 | 1 | 100% | 🟢 |
| phone_number | 1 | 1 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 1 | 100% | 🟢 |
| inchi | 1 | 1 | 100% | 🟢 |
| dms | 1 | 1 | 100% | 🟢 |
| h3 | 1 | 1 | 100% | 🟢 |
| aws_arn | 1 | 1 | 100% | 🟢 |
| isrc | 1 | 1 | 100% | 🟢 |
| loinc | 1 | 1 | 100% | 🟢 |
| snowflake_id | 1 | 1 | 100% | 🟢 |
| month_name | 1 | 1 | 100% | 🟢 |
| compact_ym | 1 | 0 | 0% | 🔴 |
| day_of_week | 1 | 1 | 100% | 🟢 |
| data_uri | 1 | 1 | 100% | 🟢 |
| username | 1 | 1 | 100% | 🟢 |
| pan_india | 1 | 1 | 100% | 🟢 |
| smiles | 1 | 1 | 100% | 🟢 |
| rfc_3339 | 1 | 1 | 100% | 🟢 |
| ulid | 1 | 1 | 100% | 🟢 |
| ein | 1 | 1 | 100% | 🟢 |
| decimal_number_comma | 1 | 1 | 100% | 🟢 |
| lei | 1 | 1 | 100% | 🟢 |
| http_method | 1 | 1 | 100% | 🟢 |
| alphanumeric_id | 1 | 1 | 100% | 🟢 |
| sql_standard | 1 | 1 | 100% | 🟢 |
| scientific_notation | 1 | 0 | 0% | 🔴 |
| region | 1 | 1 | 100% | 🟢 |
| mgrs | 1 | 1 | 100% | 🟢 |
| extension | 1 | 0 | 0% | 🔴 |
| cas_number | 1 | 1 | 100% | 🟢 |
| vin | 1 | 1 | 100% | 🟢 |
| isin | 1 | 0 | 0% | 🔴 |
| plus_code | 1 | 1 | 100% | 🟢 |
| phone_e164 | 1 | 1 | 100% | 🟢 |
| clf | 1 | 1 | 100% | 🟢 |
| ymd_dot | 1 | 1 | 100% | 🟢 |
| mdy_12h | 1 | 1 | 100% | 🟢 |
| year | 1 | 1 | 100% | 🟢 |
| iso_8601_milliseconds | 1 | 1 | 100% | 🟢 |
| hash | 1 | 0 | 0% | 🔴 |
| rfc_2822 | 1 | 1 | 100% | 🟢 |
| hm_12h | 1 | 1 | 100% | 🟢 |
| hs_code | 1 | 1 | 100% | 🟢 |
| tsid | 1 | 1 | 100% | 🟢 |
| compact_dmy | 1 | 1 | 100% | 🟢 |
| iso6346 | 1 | 1 | 100% | 🟢 |

## Actionability Evaluation

Can analysts safely TRY_CAST using FineType's format_string predictions?
**Target:** >95% success rate for datetime types

### By Type

| Type | Columns | Values | Success Rate | Status |
|---|---|---|---|---|
| integer_number | 20 | 48946 | 100% | 🟢 |
| decimal_number | 19 | 85617 | 99.9% | 🟢 |
| iso_8601 | 12 | 14882 | 96.9% | 🟢 |
| country_code | 11 | 43464 | 100% | 🟢 |
| amount | 10 | 1676 | 100% | 🟢 |
| categorical | 9 | 1564 | 100% | 🟢 |
| url | 8 | 605 | 100% | 🟢 |
| country | 7 | 56327 | 100% | 🟢 |
| ssn | 7 | 620 | 100% | 🟢 |
| entity_name | 7 | 8068 | 100% | 🟢 |
| ip_v4 | 7 | 14577 | 100% | 🟢 |
| iso | 6 | 1440 | 100% | 🟢 |
| postal_code | 6 | 445 | 100% | 🟢 |
| city | 6 | 41997 | 100% | 🟢 |
| full_address | 5 | 22015 | 100% | 🟢 |
| latitude | 5 | 22040 | 100% | 🟢 |
| longitude | 5 | 22040 | 100% | 🟢 |
| dmy_short_dot | 4 | 14201 | 0.5% | 🔴 |
| iana | 4 | 15576 | 100% | 🟢 |
| email | 4 | 285 | 100% | 🟢 |
| email_display | 4 | 300 | 100% | 🟢 |
| terms | 4 | 285 | 100% | 🟢 |
| ordinal | 4 | 1867 | 100% | 🟢 |
| periodicity | 3 | 14332 | 100% | 🟢 |
| aba_routing | 3 | 33397 | 100% | 🟢 |
| currency_code | 3 | 225 | 100% | 🟢 |
| issn | 3 | 230 | 100% | 🟢 |
| gender | 3 | 1051 | 100% | 🟢 |
| uuid | 3 | 260 | 100% | 🟢 |
| percentage | 3 | 250 | 100% | 🟢 |
| whitespace_separated | 2 | 120 | 100% | 🟢 |
| mdy_dash | 2 | 50 | 68% | 🔴 |
| utc | 2 | 7778 | 100% | 🟢 |
| iso_8601_offset | 2 | 75 | 33.3% | 🔴 |
| bsb | 2 | 180 | 100% | 🟢 |
| ethereum_address | 2 | 160 | 100% | 🟢 |
| geohash | 2 | 14212 | 100% | 🟢 |
| wkt | 2 | 160 | 100% | 🟢 |
| unlocode | 2 | 7778 | 100% | 🟢 |
| ean | 2 | 160 | 100% | 🟢 |
| abn | 2 | 140 | 100% | 🟢 |
| eu_vat | 2 | 105 | 100% | 🟢 |
| hcpcs | 2 | 140 | 100% | 🟢 |
| height | 2 | 0 | 0% | 🔴 |
| last_name | 2 | 160 | 100% | 🟢 |
| weight | 2 | 160 | 100% | 🟢 |
| file_size | 2 | 125 | 100% | 🟢 |
| increment | 2 | 991 | 100% | 🟢 |
| s3_uri | 2 | 160 | 100% | 🟢 |
| locale_code | 2 | 105 | 100% | 🟢 |
| jwt | 2 | 160 | 100% | 🟢 |
| docker_ref | 2 | 180 | 100% | 🟢 |
| snowflake_id | 2 | 80 | 100% | 🟢 |
| cidr | 2 | 105 | 100% | 🟢 |
| http_method | 2 | 125 | 100% | 🟢 |
| query_string | 1 | 100 | 100% | 🟢 |
| json_array | 1 | 0 | 0% | 🔴 |
| day_of_week | 1 | 80 | 100% | 🟢 |
| month_name | 1 | 80 | 100% | 🟢 |
| year | 1 | 60 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 80 | 0% | 🔴 |
| compact_dmy | 1 | 25 | 100% | 🟢 |
| compact_mdy | 1 | 25 | 0% | 🔴 |
| compact_ym | 1 | 80 | 0% | 🔴 |
| compact_ymd | 1 | 247 | 0% | 🔴 |
| full_month_no_comma | 1 | 80 | 0% | 🔴 |
| iso_week | 1 | 25 | 100% | 🟢 |
| ymd_dot | 1 | 25 | 100% | 🟢 |
| iso_8601 | 1 | 0 | 0% | 🔴 |
| unix_seconds | 1 | 80 | 100% | 🟢 |
| fiscal_year | 1 | 25 | 100% | 🟢 |
| quarter | 1 | 25 | 0% | 🔴 |
| hm_12h | 1 | 80 | 100% | 🟢 |
| hm_24h | 1 | 100 | 100% | 🟢 |
| hms_12h | 1 | 80 | 100% | 🟢 |
| hms_24h | 1 | 80 | 100% | 🟢 |
| clf | 1 | 25 | 0% | 🔴 |
| iso_8601_compact | 1 | 80 | 0% | 🔴 |
| iso_8601_milliseconds | 1 | 14132 | 100% | 🟢 |
| iso_microseconds | 1 | 60 | 0% | 🔴 |
| mdy_12h | 1 | 80 | 100% | 🟢 |
| rfc_2822 | 1 | 80 | 100% | 🟢 |
| rfc_3339 | 1 | 0 | 0% | 🔴 |
| sql_standard | 1 | 80 | 100% | 🟢 |
| syslog_bsd | 1 | 0 | 0% | 🔴 |
| iban | 1 | 25 | 100% | 🟢 |
| swift_bic | 1 | 50 | 100% | 🟢 |
| amount_nodecimal | 1 | 60 | 66.7% | 🔴 |
| credit_card_number | 1 | 80 | 100% | 🟢 |
| figi | 1 | 80 | 100% | 🟢 |
| isin | 1 | 80 | 100% | 🟢 |
| lei | 1 | 25 | 100% | 🟢 |
| coordinates | 1 | 0 | 0% | 🔴 |
| dms | 1 | 80 | 100% | 🟢 |
| mgrs | 1 | 80 | 100% | 🟢 |
| plus_code | 1 | 80 | 100% | 🟢 |
| h3 | 1 | 80 | 100% | 🟢 |
| region | 1 | 249 | 100% | 🟢 |
| hs_code | 1 | 80 | 100% | 🟢 |
| iata_code | 1 | 7698 | 100% | 🟢 |
| icao_code | 1 | 100 | 100% | 🟢 |
| iso6346 | 1 | 80 | 100% | 🟢 |
| orcid | 1 | 80 | 100% | 🟢 |
| isrc | 1 | 80 | 100% | 🟢 |
| ein | 1 | 80 | 100% | 🟢 |
| pan_india | 1 | 80 | 100% | 🟢 |
| vin | 1 | 80 | 100% | 🟢 |
| cpt | 1 | 80 | 100% | 🟢 |
| dea_number | 1 | 249 | 100% | 🟢 |
| icd10 | 1 | 80 | 100% | 🟢 |
| loinc | 1 | 80 | 100% | 🟢 |
| npi | 1 | 60 | 100% | 🟢 |
| first_name | 1 | 100 | 100% | 🟢 |
| full_name | 1 | 60 | 100% | 🟢 |
| gender_code | 1 | 14132 | 100% | 🟢 |
| phone_e164 | 1 | 80 | 100% | 🟢 |
| phone_number | 1 | 60 | 100% | 🟢 |
| username | 1 | 25 | 100% | 🟢 |
| binary | 1 | 891 | 100% | 🟢 |
| extension | 1 | 60 | 100% | 🟢 |
| color_hex | 1 | 80 | 100% | 🟢 |
| color_hsl | 1 | 80 | 100% | 🟢 |
| alphanumeric_id | 1 | 25 | 100% | 🟢 |
| numeric_code | 1 | 247 | 100% | 🟢 |
| decimal_number_comma | 1 | 25 | 100% | 🟢 |
| scientific_notation | 1 | 150 | 100% | 🟢 |
| cas_number | 1 | 80 | 100% | 🟢 |
| inchi | 1 | 80 | 100% | 🟢 |
| smiles | 1 | 80 | 100% | 🟢 |
| aws_arn | 1 | 80 | 100% | 🟢 |
| hash | 1 | 25 | 100% | 🟢 |
| token_urlsafe | 1 | 80 | 100% | 🟢 |
| calver | 1 | 25 | 100% | 🟢 |
| tsid | 1 | 80 | 100% | 🟢 |
| ulid | 1 | 80 | 100% | 🟢 |
| data_uri | 1 | 80 | 100% | 🟢 |
| mac_address | 1 | 80 | 100% | 🟢 |
| urn | 1 | 80 | 100% | 🟢 |

### Below Target (<95%)

| Dataset | Column | Type | Format | Success Rate |
|---|---|---|---|---|
| countries | region-code | compact_ymd | `` | 0% 🔴 |
| datetime_formats | us_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats | eu_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats | unix_ms | iso_8601_compact | `` | 0% 🔴 |
| datetime_formats | year | compact_ym | `` | 0% 🔴 |
| medical_records | patient_id | iso_microseconds | `` | 0% 🔴 |
| scientific_measurements | ph_value | dmy_short_dot | `` | 0% 🔴 |
| scientific_measurements | timestamp | iso_8601_offset | `` | 0% 🔴 |
| multilingual | date | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | eu_dot_date | iso_8601 | `` | 0% 🔴 |
| datetime_formats_extended | abbreviated_month_date | abbrev_month_no_comma | `` | 0% 🔴 |
| datetime_formats_extended | long_full_month_date | full_month_no_comma | `` | 0% 🔴 |
| datetime_formats_extended | european_timestamp | iso_8601 | `` | 0% 🔴 |
| ecommerce_orders_json | order_date | iso_8601 | `` | 0% 🔴 |
| weather_stations_json | observation_date | iso_8601 | `` | 0% 🔴 |
| weather_stations_json | precipitation_mm | dmy_short_dot | `` | 0% 🔴 |
| earthquakes_2024 | magError | dmy_short_dot | `` | 0% 🔴 |
| datetime_coverage | compact_ymd | compact_mdy | `` | 0% 🔴 |
| datetime_coverage | rfc_3339 | rfc_3339 | `` | 0% 🔴 |
| datetime_coverage | clf_timestamp | clf | `` | 0% 🔴 |
| datetime_coverage | syslog_bsd | syslog_bsd | `` | 0% 🔴 |
| people_directory | height_cm | height | `` | 0% 🔴 |
| datetime_formats | duration_iso | iso_8601 | `` | 0% 🔴 |
| geography_data | coordinates | coordinates | `` | 0% 🔴 |
| codes_and_ids | iban | snowflake_id | `` | 0% 🔴 |
| medical_records | blood_pressure | decimal_number | `` | 0% 🔴 |
| medical_records | height_in | height | `` | 0% 🔴 |
| server_logs_json | user_agent | json_array | `` | 0% 🔴 |
| datetime_coverage | quarter | quarter | `` | 0% 🔴 |
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
