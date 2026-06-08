# FineType Evaluation Report

**Generated:** 2026-04-21 13:35

## Headline Metrics

| Metric | Value | Status |
|---|---|---|
| Profile label accuracy | 297/352 (84.4%) | 🟡 |
| Profile domain accuracy | 323/352 (91.8%) | 🟡 |
| Actionability (datetime) | 579440/579554 (100%) | 🟢 |
| Columns with >95% parse rate | 423/441 | |
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

**Label accuracy:** 297/352 (84.4%)
**Domain accuracy:** 323/352 (91.8%)

### Misclassifications

| Dataset | Column | Predicted | Expected | Confidence |
|---|---|---|---|---|
| coverage_closure_phase_ab | amount_nodecimal | amount | amount_nodecimal | 1.00 |
| coverage_closure_phase_ab | yield | percentage | yield | 1.00 |
| tech_systems | user_agent | jwt | user_agent | 1.00 |
| coverage_closure_phase_ab | ethereum_address | full_address | ethereum_address | 1.00 |
| coverage_closure_phase_ab | iso_microseconds | sql_microseconds | iso_microseconds | 1.00 |
| coverage_closure_phase_ab | unix_microseconds | unix_seconds | unix_microseconds | 1.00 |
| coverage_closure_phase_ab | amount_code_prefix | amount | amount_code_prefix | 0.99 |
| new_geography | geojson | plain_text | json | 0.99 |
| coverage_closure_phase_ab | street_name | full_name | street_name | 0.98 |
| people_directory | phone | ssn | phone_number | 0.97 |
| coverage_closure_phase_ab | json_array | categorical | json_array | 0.94 |
| coverage_closure_phase_ab | query_string | categorical | query_string | 0.94 |
| coverage_closure_phase_ab | csv | categorical | csv | 0.92 |
| coverage_closure_phase_ab | amount_comma_suffix | amount | amount_comma_suffix | 0.92 |
| coverage_closure_phase_ab | xml | categorical | xml | 0.91 |
| coverage_closure_phase_ab | numeric_code | integer_number | numeric_code | 0.91 |
| coverage_closure_phase_ab | street_suffix | street_address | street_suffix | 0.91 |
| coverage_closure_phase_ab | jp_era_short | alphanumeric_id | jp_era_short | 0.89 |
| coverage_closure_phase_ab | si_number | file_size | si_number | 0.87 |
| earthquakes_2024 | id | username | alphanumeric_id | 0.86 |
| coverage_closure_phase_ab | state_code | region | state_code | 0.86 |
| coverage_closure_phase_ab | html | categorical | html | 0.86 |
| coverage_closure_phase_ab | iso_8601_compact | alphanumeric_id | iso_8601_compact | 0.84 |
| coverage_closure_phase_ab | plain_text | categorical | plain_text | 0.84 |
| coverage_closure_phase_ab | short_dmy | dmy_slash | short_dmy | 0.84 |
| coverage_closure_phase_ab | word | categorical | word | 0.81 |
| coverage_closure_phase_ab | discrete_ordinal | categorical | ordinal | 0.80 |
| coverage_closure_phase_ab | yaml | categorical | yaml | 0.74 |
| coverage_closure_phase_ab | whitespace_separated | entity_name | whitespace_separated | 0.73 |
| coverage_closure_phase_ab | calling_code | plain_text | calling_code | 0.71 |
| coverage_closure_phase_ab | julian | integer_number | julian | 0.70 |
| coverage_closure_phase_ab | iso_8601_milliseconds | categorical | iso_8601_milliseconds | 0.70 |
| coverage_closure_phase_ab | amount_neg_trailing | amount | amount_neg_trailing | 0.65 |
| coverage_closure_phase_ab | short_mdy | mdy_slash | short_mdy | 0.65 |
| coverage_closure_phase_ab | short_ymd | ymd_slash | short_ymd | 0.64 |
| coverage_closure_phase_ab | amount_crypto | amount | amount_crypto | 0.61 |
| coverage_closure_phase_ab | gender_code | categorical | gender_code | 0.60 |
| coverage_closure_phase_ab | amount_space | amount | amount_space | 0.56 |
| coverage_closure_phase_ab | measurement_unit | entity_name | measurement_unit | 0.54 |
| coverage_closure_phase_ab | semicolon_separated | categorical | semicolon_separated | 0.51 |
| datetime_coverage | fiscal_year | year | fiscal_year | 0.50 |
| coverage_closure_phase_ab | amount_accounting | amount | amount_accounting | 0.50 |
| coverage_closure_phase_ab | amount_apostrophe | amount | amount_apostrophe | 0.50 |
| coverage_closure_phase_ab | amount_comma | amount | amount_comma | 0.50 |
| coverage_closure_phase_ab | amount_lakh | amount | amount_lakh | 0.50 |
| coverage_closure_phase_ab | amount_multisym | amount | amount_multisym | 0.50 |
| coverage_closure_phase_ab | password | password | password | 0.50 |
| network_logs | user_agent | docker_ref | user_agent | 0.42 |
| coverage_closure_phase_ab | ordinal | abbreviated_month | ordinal | 0.38 |
| coverage_closure_phase_ab | dna_sequence | entity_name | dna_sequence | 0.38 |
| coverage_closure_phase_ab | sedol | alphanumeric_id | sedol | 0.35 |
| coverage_closure_phase_ab | dot_dmy_24h | sql_milliseconds | dot_dmy_24h | 0.34 |
| coverage_closure_phase_ab | pg_short_offset | categorical | pg_short_offset | 0.32 |
| coverage_closure_phase_ab | excel_format | categorical | excel_format | 0.32 |
| multilingual | locale | categorical | locale_code | 0.26 |

## Precision Per Type (Profile Eval)

| Predicted Type | Predicted | Correct | Precision | Status |
|---|---|---|---|---|
| decimal_number | 23 | 23 | 100% | 🟢 |
| categorical | 17 | 2 | 11.8% | 🔴 |
| amount | 12 | 1 | 8.3% | 🔴 |
| iso | 9 | 9 | 100% | 🟢 |
| country | 8 | 8 | 100% | 🟢 |
| integer_number | 7 | 5 | 71.4% | 🔴 |
| entity_name | 7 | 4 | 57.1% | 🔴 |
| full_name | 6 | 5 | 83.3% | 🟡 |
| full_address | 5 | 4 | 80% | 🟡 |
| longitude | 5 | 5 | 100% | 🟢 |
| country_code | 5 | 5 | 100% | 🟢 |
| city | 5 | 5 | 100% | 🟢 |
| terms | 5 | 5 | 100% | 🟢 |
| url | 5 | 5 | 100% | 🟢 |
| latitude | 5 | 5 | 100% | 🟢 |
| email | 4 | 4 | 100% | 🟢 |
| alphanumeric_id | 4 | 1 | 25% | 🔴 |
| ip_v4 | 4 | 4 | 100% | 🟢 |
| percentage | 4 | 3 | 75% | 🔴 |
| iso_8601_milliseconds | 4 | 4 | 100% | 🟢 |
| postal_code | 4 | 4 | 100% | 🟢 |
| phone_number | 3 | 3 | 100% | 🟢 |
| year | 3 | 2 | 66.7% | 🔴 |
| ssn | 3 | 2 | 66.7% | 🔴 |
| uuid | 3 | 3 | 100% | 🟢 |
| gender | 3 | 3 | 100% | 🟢 |
| continent | 3 | 3 | 100% | 🟢 |
| region | 3 | 2 | 66.7% | 🔴 |
| icd10 | 2 | 2 | 100% | 🟢 |
| iana | 2 | 2 | 100% | 🟢 |
| currency_code | 2 | 2 | 100% | 🟢 |
| abbreviated_month | 2 | 1 | 50% | 🔴 |
| sql_microseconds | 2 | 1 | 50% | 🔴 |
| dmy_slash | 2 | 1 | 50% | 🔴 |
| weight | 2 | 2 | 100% | 🟢 |
| first_name | 2 | 2 | 100% | 🟢 |
| last_name | 2 | 2 | 100% | 🟢 |
| mdy_slash | 2 | 1 | 50% | 🔴 |
| file_size | 2 | 1 | 50% | 🔴 |
| height | 2 | 2 | 100% | 🟢 |
| hms_24h | 2 | 2 | 100% | 🟢 |
| sql_milliseconds | 2 | 1 | 50% | 🔴 |
| jwt | 2 | 1 | 50% | 🔴 |
| http_method | 2 | 2 | 100% | 🟢 |
| rfc_2822 | 2 | 2 | 100% | 🟢 |
| plain_text | 2 | 0 | 0% | 🔴 |
| utc | 2 | 2 | 100% | 🟢 |
| ymd_slash | 2 | 1 | 50% | 🔴 |
| dmy_dash | 2 | 2 | 100% | 🟢 |
| docker_ref | 2 | 1 | 50% | 🔴 |
| hash | 2 | 2 | 100% | 🟢 |
| username | 2 | 1 | 50% | 🔴 |
| unix_seconds | 2 | 1 | 50% | 🔴 |
| ip_v4_with_port | 2 | 2 | 100% | 🟢 |
| issn | 1 | 1 | 100% | 🟢 |
| tsid | 1 | 1 | 100% | 🟢 |
| loinc | 1 | 1 | 100% | 🟢 |
| lei | 1 | 1 | 100% | 🟢 |
| state | 1 | 1 | 100% | 🟢 |
| smiles | 1 | 1 | 100% | 🟢 |
| month_year_full | 1 | 1 | 100% | 🟢 |
| initials | 1 | 1 | 100% | 🟢 |
| doi | 1 | 1 | 100% | 🟢 |
| iso_8601_offset | 1 | 1 | 100% | 🟢 |
| imei | 1 | 1 | 100% | 🟢 |
| korean_ymd | 1 | 1 | 100% | 🟢 |
| token_urlsafe | 1 | 1 | 100% | 🟢 |
| cidr | 1 | 1 | 100% | 🟢 |
| isin | 1 | 1 | 100% | 🟢 |
| color_hsl | 1 | 1 | 100% | 🟢 |
| npi | 1 | 1 | 100% | 🟢 |
| h3 | 1 | 1 | 100% | 🟢 |
| iso_space_zulu | 1 | 1 | 100% | 🟢 |
| user_agent | 1 | 1 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 1 | 100% | 🟢 |
| locale_code | 1 | 1 | 100% | 🟢 |
| protein_sequence | 1 | 1 | 100% | 🟢 |
| dmy_space_full | 1 | 1 | 100% | 🟢 |
| phone_e164 | 1 | 1 | 100% | 🟢 |
| s3_uri | 1 | 1 | 100% | 🟢 |
| icao_code | 1 | 1 | 100% | 🟢 |
| swift_bic | 1 | 1 | 100% | 🟢 |
| compact_dmy | 1 | 1 | 100% | 🟢 |
| decimal_number_comma | 1 | 1 | 100% | 🟢 |
| bitcoin_address | 1 | 1 | 100% | 🟢 |
| mdy_short_slash | 1 | 1 | 100% | 🟢 |
| rfc_3339 | 1 | 1 | 100% | 🟢 |
| measurement_unit | 1 | 1 | 100% | 🟢 |
| day_of_week | 1 | 1 | 100% | 🟢 |
| pan_india | 1 | 1 | 100% | 🟢 |
| dmy_dot | 1 | 1 | 100% | 🟢 |
| unlocode | 1 | 1 | 100% | 🟢 |
| pipe_separated | 1 | 1 | 100% | 🟢 |
| ctime | 1 | 1 | 100% | 🟢 |
| comma_separated | 1 | 1 | 100% | 🟢 |
| slash_ymd_24h | 1 | 1 | 100% | 🟢 |
| emoji | 1 | 1 | 100% | 🟢 |
| iata_code | 1 | 1 | 100% | 🟢 |
| mac_address | 1 | 1 | 100% | 🟢 |
| color_rgb | 1 | 1 | 100% | 🟢 |
| dmy_dash_abbrev | 1 | 1 | 100% | 🟢 |
| weekday_abbreviated_month | 1 | 1 | 100% | 🟢 |
| ean | 1 | 1 | 100% | 🟢 |
| compact_ymd | 1 | 1 | 100% | 🟢 |
| chinese_ymd | 1 | 1 | 100% | 🟢 |
| vin | 1 | 1 | 100% | 🟢 |
| email_display | 1 | 1 | 100% | 🟢 |
| data_uri | 1 | 1 | 100% | 🟢 |
| password | 1 | 0 | 0% | 🔴 |
| mgrs | 1 | 1 | 100% | 🟢 |
| iso_8601_millis_offset | 1 | 1 | 100% | 🟢 |
| isrc | 1 | 1 | 100% | 🟢 |
| mdy_12h | 1 | 1 | 100% | 🟢 |
| ulid | 1 | 1 | 100% | 🟢 |
| long_full_month | 1 | 1 | 100% | 🟢 |
| unix_milliseconds | 1 | 1 | 100% | 🟢 |
| geohash | 1 | 1 | 100% | 🟢 |
| figi | 1 | 1 | 100% | 🟢 |
| upc | 1 | 1 | 100% | 🟢 |
| cas_number | 1 | 1 | 100% | 🟢 |
| full_month_no_comma | 1 | 1 | 100% | 🟢 |
| blood_type | 1 | 1 | 100% | 🟢 |
| aba_routing | 1 | 1 | 100% | 🟢 |
| eu_vat | 1 | 1 | 100% | 🟢 |
| month_year_slash | 1 | 1 | 100% | 🟢 |
| hm_12h | 1 | 1 | 100% | 🟢 |
| hms_12h | 1 | 1 | 100% | 🟢 |
| binary | 1 | 1 | 100% | 🟢 |
| cpt | 1 | 1 | 100% | 🟢 |
| scientific_notation | 1 | 1 | 100% | 🟢 |
| abn | 1 | 1 | 100% | 🟢 |
| weekday_dmy_full | 1 | 1 | 100% | 🟢 |
| iso_week | 1 | 1 | 100% | 🟢 |
| year_month | 1 | 1 | 100% | 🟢 |
| dot_ymd_24h | 1 | 1 | 100% | 🟢 |
| epoch_nanoseconds | 1 | 1 | 100% | 🟢 |
| rna_sequence | 1 | 1 | 100% | 🟢 |
| dms | 1 | 1 | 100% | 🟢 |
| dmy_hm | 1 | 1 | 100% | 🟢 |
| iso6346 | 1 | 1 | 100% | 🟢 |
| hostname | 1 | 1 | 100% | 🟢 |
| aws_arn | 1 | 1 | 100% | 🟢 |
| compact_mdy | 1 | 1 | 100% | 🟢 |
| dmy_space_abbrev | 1 | 1 | 100% | 🟢 |
| dmy_dash_abbrev_short | 1 | 1 | 100% | 🟢 |
| jp_era_long | 1 | 1 | 100% | 🟢 |
| bsb | 1 | 1 | 100% | 🟢 |
| basis_points | 1 | 1 | 100% | 🟢 |
| plus_code | 1 | 1 | 100% | 🟢 |
| ndc | 1 | 1 | 100% | 🟢 |
| dea_number | 1 | 1 | 100% | 🟢 |
| iban | 1 | 1 | 100% | 🟢 |
| inchi | 1 | 1 | 100% | 🟢 |
| month_year_abbrev | 1 | 1 | 100% | 🟢 |
| iso_8601_micros_offset | 1 | 1 | 100% | 🟢 |
| sql_microseconds_offset | 1 | 1 | 100% | 🟢 |
| calver | 1 | 1 | 100% | 🟢 |
| extension | 1 | 1 | 100% | 🟢 |
| hcpcs | 1 | 1 | 100% | 🟢 |
| street_address | 1 | 0 | 0% | 🔴 |
| top_level_domain | 1 | 1 | 100% | 🟢 |
| wkt | 1 | 1 | 100% | 🟢 |
| credit_card_number | 1 | 1 | 100% | 🟢 |
| month_name | 1 | 1 | 100% | 🟢 |
| cusip | 1 | 1 | 100% | 🟢 |
| iso_microseconds | 1 | 1 | 100% | 🟢 |
| snowflake_id | 1 | 1 | 100% | 🟢 |
| syslog_bsd | 1 | 1 | 100% | 🟢 |
| iso_8601 | 1 | 1 | 100% | 🟢 |
| orcid | 1 | 1 | 100% | 🟢 |
| hs_code | 1 | 1 | 100% | 🟢 |
| periodicity | 1 | 1 | 100% | 🟢 |
| weekday_full_month | 1 | 1 | 100% | 🟢 |
| ymd_dot | 1 | 1 | 100% | 🟢 |
| urn | 1 | 1 | 100% | 🟢 |
| sql_standard | 1 | 1 | 100% | 🟢 |
| ein | 1 | 1 | 100% | 🟢 |
| quarter | 1 | 1 | 100% | 🟢 |
| hm_24h | 1 | 1 | 100% | 🟢 |
| ip_v6 | 1 | 1 | 100% | 🟢 |
| dmy_short_dot | 1 | 1 | 100% | 🟢 |
| compact_ym | 1 | 1 | 100% | 🟢 |
| mdy_24h | 1 | 1 | 100% | 🟢 |
| clf | 1 | 1 | 100% | 🟢 |
| dmy_short_slash | 1 | 1 | 100% | 🟢 |

## Actionability Evaluation

Can analysts safely TRY_CAST using FineType's format_string predictions?
**Target:** >95% success rate for datetime types

### By Type

| Type | Columns | Values | Success Rate | Status |
|---|---|---|---|---|
| categorical | 26 | 1854 | 100% | 🟢 |
| integer_number | 26 | 96455 | 100% | 🟢 |
| amount | 23 | 1808 | 99.7% | 🟢 |
| decimal_number | 23 | 99940 | 100% | 🟢 |
| entity_name | 11 | 8370 | 100% | 🟢 |
| iso | 9 | 1660 | 100% | 🟢 |
| country | 8 | 42544 | 100% | 🟢 |
| alphanumeric_id | 8 | 353 | 100% | 🟢 |
| city | 6 | 55238 | 100% | 🟢 |
| country_code | 6 | 967 | 100% | 🟢 |
| full_name | 6 | 1177 | 100% | 🟢 |
| url | 6 | 425 | 100% | 🟢 |
| full_address | 5 | 21996 | 100% | 🟢 |
| postal_code | 5 | 420 | 100% | 🟢 |
| latitude | 5 | 22040 | 100% | 🟢 |
| longitude | 5 | 22040 | 100% | 🟢 |
| terms | 5 | 291 | 99.3% | 🟢 |
| measurement_unit | 5 | 56578 | 100% | 🟢 |
| iso_8601 | 4 | 235 | 100% | 🟢 |
| iso_8601_milliseconds | 4 | 28389 | 100% | 🟢 |
| currency_code | 4 | 250 | 100% | 🟢 |
| email | 4 | 285 | 100% | 🟢 |
| ordinal | 4 | 1867 | 100% | 🟢 |
| percentage | 4 | 256 | 100% | 🟢 |
| ip_v4 | 4 | 305 | 100% | 🟢 |
| year | 3 | 140 | 100% | 🟢 |
| ymd_slash | 3 | 72 | 44.4% | 🔴 |
| unix_seconds | 3 | 86 | 100% | 🟢 |
| hms_24h | 3 | 92 | 93.5% | 🟡 |
| continent | 3 | 500 | 100% | 🟢 |
| region | 3 | 33220 | 100% | 🟢 |
| ssn | 3 | 280 | 100% | 🟢 |
| gender | 3 | 1051 | 100% | 🟢 |
| phone_number | 3 | 220 | 100% | 🟢 |
| uuid | 3 | 260 | 100% | 🟢 |
| plain_text | 3 | 111 | 100% | 🟢 |
| abbreviated_month | 2 | 86 | 93% | 🟡 |
| dmy_dash | 2 | 50 | 68% | 🔴 |
| dmy_short_dot | 2 | 86 | 94.2% | 🟡 |
| dmy_slash | 2 | 86 | 100% | 🟢 |
| mdy_slash | 2 | 86 | 100% | 🟢 |
| unix_milliseconds | 2 | 86 | 100% | 🟢 |
| iana | 2 | 7778 | 100% | 🟢 |
| utc | 2 | 7778 | 100% | 🟢 |
| hm_24h | 2 | 106 | 100% | 🟢 |
| rfc_2822 | 2 | 86 | 93% | 🟡 |
| sql_microseconds | 2 | 12 | 100% | 🟢 |
| sql_milliseconds | 2 | 12 | 50% | 🔴 |
| icao_code | 2 | 7798 | 100% | 🟢 |
| ean | 2 | 180 | 100% | 🟢 |
| isbn | 2 | 140 | 100% | 🟢 |
| icd10 | 2 | 140 | 100% | 🟢 |
| first_name | 2 | 160 | 100% | 🟢 |
| height | 2 | 160 | 100% | 🟢 |
| last_name | 2 | 160 | 100% | 🟢 |
| username | 2 | 14157 | 100% | 🟢 |
| weight | 2 | 160 | 100% | 🟢 |
| file_size | 2 | 31 | 80.6% | 🟡 |
| mime_type | 2 | 180 | 100% | 🟢 |
| numeric_code | 2 | 494 | 100% | 🟢 |
| smiles | 2 | 130 | 100% | 🟢 |
| word | 2 | 21830 | 100% | 🟢 |
| hash | 2 | 160 | 100% | 🟢 |
| jwt | 2 | 160 | 100% | 🟢 |
| token_urlsafe | 2 | 86 | 100% | 🟢 |
| docker_ref | 2 | 180 | 100% | 🟢 |
| http_method | 2 | 125 | 100% | 🟢 |
| ip_v4_with_port | 2 | 31 | 100% | 🟢 |
| comma_separated | 1 | 6 | 100% | 🟢 |
| pipe_separated | 1 | 6 | 100% | 🟢 |
| whitespace_separated | 1 | 60 | 100% | 🟢 |
| query_string | 1 | 100 | 100% | 🟢 |
| day_of_week | 1 | 80 | 100% | 🟢 |
| month_name | 1 | 80 | 100% | 🟢 |
| periodicity | 1 | 6 | 100% | 🟢 |
| abbrev_month_no_comma | 1 | 6 | 100% | 🟢 |
| chinese_ymd | 1 | 6 | 100% | 🟢 |
| compact_dmy | 1 | 25 | 100% | 🟢 |
| compact_mdy | 1 | 6 | 100% | 🟢 |
| compact_ym | 1 | 6 | 100% | 🟢 |
| compact_ymd | 1 | 25 | 100% | 🟢 |
| dmy_dash_abbrev | 1 | 6 | 100% | 🟢 |
| dmy_dash_abbrev_short | 1 | 6 | 100% | 🟢 |
| dmy_dot | 1 | 80 | 100% | 🟢 |
| dmy_short_slash | 1 | 6 | 100% | 🟢 |
| dmy_space_abbrev | 1 | 6 | 100% | 🟢 |
| dmy_space_full | 1 | 6 | 100% | 🟢 |
| full_month_no_comma | 1 | 6 | 100% | 🟢 |
| iso_week | 1 | 25 | 100% | 🟢 |
| jp_era_long | 1 | 0 | 0% | 🔴 |
| korean_ymd | 1 | 6 | 100% | 🟢 |
| long_full_month | 1 | 80 | 100% | 🟢 |
| mdy_short_slash | 1 | 6 | 100% | 🟢 |
| month_year_abbrev | 1 | 6 | 100% | 🟢 |
| month_year_full | 1 | 6 | 100% | 🟢 |
| month_year_slash | 1 | 6 | 100% | 🟢 |
| weekday_abbreviated_month | 1 | 6 | 0% | 🔴 |
| weekday_dmy_full | 1 | 6 | 100% | 🟢 |
| weekday_full_month | 1 | 6 | 100% | 🟢 |
| year_month | 1 | 6 | 100% | 🟢 |
| ymd_dot | 1 | 25 | 100% | 🟢 |
| iso_8601 | 1 | 80 | 100% | 🟢 |
| quarter | 1 | 25 | 100% | 🟢 |
| hm_12h | 1 | 80 | 100% | 🟢 |
| hms_12h | 1 | 80 | 100% | 🟢 |
| clf | 1 | 25 | 100% | 🟢 |
| ctime | 1 | 6 | 100% | 🟢 |
| dmy_hm | 1 | 80 | 100% | 🟢 |
| dot_ymd_24h | 1 | 6 | 0% | 🔴 |
| epoch_nanoseconds | 1 | 0 | 0% | 🔴 |
| iso_8601_micros_offset | 1 | 6 | 83.3% | 🟡 |
| iso_8601_millis_offset | 1 | 6 | 83.3% | 🟡 |
| iso_8601_offset | 1 | 25 | 100% | 🟢 |
| iso_microseconds | 1 | 6 | 100% | 🟢 |
| iso_space_zulu | 1 | 6 | 100% | 🟢 |
| mdy_12h | 1 | 80 | 100% | 🟢 |
| mdy_24h | 1 | 6 | 100% | 🟢 |
| rfc_3339 | 1 | 25 | 100% | 🟢 |
| slash_ymd_24h | 1 | 6 | 100% | 🟢 |
| sql_microseconds_offset | 1 | 6 | 100% | 🟢 |
| sql_standard | 1 | 80 | 100% | 🟢 |
| syslog_bsd | 1 | 25 | 96% | 🟢 |
| aba_routing | 1 | 80 | 100% | 🟢 |
| bsb | 1 | 80 | 100% | 🟢 |
| iban | 1 | 80 | 100% | 🟢 |
| swift_bic | 1 | 80 | 100% | 🟢 |
| bitcoin_address | 1 | 25 | 100% | 🟢 |
| credit_card_number | 1 | 80 | 100% | 🟢 |
| basis_points | 1 | 6 | 100% | 🟢 |
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
| dea_number | 1 | 6 | 100% | 🟢 |
| hcpcs | 1 | 80 | 100% | 🟢 |
| loinc | 1 | 80 | 100% | 🟢 |
| ndc | 1 | 6 | 100% | 🟢 |
| npi | 1 | 60 | 100% | 🟢 |
| blood_type | 1 | 6 | 100% | 🟢 |
| email_display | 1 | 80 | 100% | 🟢 |
| phone_e164 | 1 | 80 | 100% | 🟢 |
| binary | 1 | 891 | 100% | 🟢 |
| initials | 1 | 6 | 100% | 🟢 |
| extension | 1 | 6 | 100% | 🟢 |
| color_hex | 1 | 80 | 100% | 🟢 |
| color_hsl | 1 | 80 | 100% | 🟢 |
| color_rgb | 1 | 6 | 100% | 🟢 |
| increment | 1 | 891 | 100% | 🟢 |
| decimal_number_comma | 1 | 25 | 100% | 🟢 |
| scientific_notation | 1 | 25 | 100% | 🟢 |
| si_number | 1 | 100 | 100% | 🟢 |
| cas_number | 1 | 80 | 100% | 🟢 |
| inchi | 1 | 80 | 100% | 🟢 |
| protein_sequence | 1 | 6 | 100% | 🟢 |
| rna_sequence | 1 | 6 | 100% | 🟢 |
| emoji | 1 | 6 | 100% | 🟢 |
| aws_arn | 1 | 80 | 100% | 🟢 |
| s3_uri | 1 | 80 | 100% | 🟢 |
| doi | 1 | 6 | 100% | 🟢 |
| imei | 1 | 6 | 100% | 🟢 |
| locale_code | 1 | 80 | 100% | 🟢 |
| calver | 1 | 25 | 100% | 🟢 |
| version | 1 | 80 | 100% | 🟢 |
| snowflake_id | 1 | 80 | 100% | 🟢 |
| tsid | 1 | 80 | 100% | 🟢 |
| ulid | 1 | 80 | 100% | 🟢 |
| cidr | 1 | 80 | 100% | 🟢 |
| data_uri | 1 | 80 | 100% | 🟢 |
| hostname | 1 | 80 | 100% | 🟢 |
| ip_v6 | 1 | 25 | 100% | 🟢 |
| mac_address | 1 | 80 | 100% | 🟢 |
| top_level_domain | 1 | 6 | 100% | 🟢 |
| urn | 1 | 80 | 100% | 🟢 |
| user_agent | 1 | 25 | 100% | 🟢 |

### Below Target (<95%)

| Dataset | Column | Type | Format | Success Rate |
|---|---|---|---|---|
| coverage_closure_phase_ab | ordinal | abbreviated_month | `` | 0% 🔴 |
| coverage_closure_phase_ab | weekday_abbreviated_month | weekday_abbreviated_month | `` | 0% 🔴 |
| coverage_closure_phase_ab | iso | hms_24h | `` | 0% 🔴 |
| coverage_closure_phase_ab | dot_dmy_24h | sql_milliseconds | `` | 0% 🔴 |
| coverage_closure_phase_ab | dot_ymd_24h | dot_ymd_24h | `` | 0% 🔴 |
| coverage_closure_phase_ab | rfc_2822_ordinal | rfc_2822 | `` | 0% 🔴 |
| datetime_coverage | fiscal_year | year | `` | 0% 🔴 |
| coverage_closure_phase_ab | jp_era_long | jp_era_long | `` | 0% 🔴 |
| coverage_closure_phase_ab | unix_microseconds | unix_seconds | `` | 0% 🔴 |
| coverage_closure_phase_ab | epoch_nanoseconds | epoch_nanoseconds | `` | 0% 🔴 |
| coverage_closure_phase_ab | amount_neg_trailing | amount | `` | 0% 🔴 |
| coverage_closure_phase_ab | si_number | file_size | `` | 0% 🔴 |
| multilingual | date | ymd_slash | `` | 33.3% 🔴 |
| datetime_coverage | mdy_dash | dmy_dash | `` | 36% 🔴 |
| coverage_closure_phase_ab | terms | terms | `` | 66.7% 🔴 |
| coverage_closure_phase_ab | iso_8601_micros_offset | iso_8601_micros_offset | `` | 83.3% 🔴 |
| coverage_closure_phase_ab | iso_8601_millis_offset | iso_8601_millis_offset | `` | 83.3% 🔴 |
| tech_systems | version | dmy_short_dot | `` | 93.8% 🔴 |

## Evaluation Components

| Component | Scope | Target | Status |
|---|---|---|---|
| Profile regression | 352 columns, 36 datasets | No regressions | 🟡 |
| Precision per type | SOTAB/GitTables | 🟢≥95% per type | Run `make eval-sotab-cli` |
| Overcall analysis | SOTAB/GitTables | <5% FP rate | Run `make eval-sotab-cli` |
| Actionability | Profile eval datetime | >95% parse rate | 🟢 |
| Confidence calibration | SOTAB/GitTables | Gap <10pp | Run `make eval-sotab-cli` |
| Domain accuracy | SOTAB format-detectable | >80% | Run `make eval-sotab-cli` |

---

## Coverage-origin delta (ac-12, eval-expansion Phase A+B)

Split of the 448-row expanded-eval profile against coverage origin:

| Bucket | Columns | Label matches | Accuracy |
|---|---|---|---|
| previously_covered | 338 | 250 | 74.0% |
| newly_covered | 110 | 58 | 52.7% |
| **combined** | **448** | **308** | **68.8%** |

Note: the combined accuracy here differs from the headline 297/352
above because the headline is computed over the scored subset (352
columns whose predictions resolve under schema_mapping.yaml), whereas
this split is over the full 448-row manifest. The point stands:
newly_covered types (added under ac-05 as zero-coverage closure) score
markedly lower than previously_covered. That drop is diagnostic — v16
was never trained against those types.

Full per-type delta for the newly_covered bucket:
`eval/eval_output/delta_by_coverage.md`
(regenerate with `python3 scripts/eval_delta_by_coverage.py`).

**ac-12 verification:** this re-score is explicitly diagnostic-only,
NOT a v18 promotion baseline. See MADR 0054 §Hold-v17 for the pattern
this programme closes.

---
*Generated by eval-report (Rust port of eval_report.py)*
