# v16 → v18 seed 42 per-column diff (expanded 352-col eval)

Spec: orbit/specs/2026-04-21-v18-retrain/spec.yaml (ac-07, ac-09)
Winner: seed 42 at 297/352 (val_acc 0.9134)

## Summary

- **Fixes** (v16 wrong → v18 correct): 8
- **Regressions** (v16 correct → v18 wrong): 8
- **Persistent misses** (both wrong): 47
  - Same prediction in both: 44 (beyond-retrain levers needed)
  - Different prediction (churn): 3
- **Net label-accuracy delta:** +0 columns
- v16 total 297/352; v18 total 297/352 (tie)

## Fixes

| Dataset | Column | gt_label | v16_label |
|---|---|---|---|
| coverage_closure_phase_ab | dna_sequence | representation.scientific.dna_sequence | entity_name |
| coverage_closure_phase_ab | dot_dmy_24h | datetime.timestamp.dot_dmy_24h | sql_milliseconds |
| coverage_closure_phase_ab | iso_8601_compact | datetime.timestamp.iso_8601_compact | alphanumeric_id |
| coverage_closure_phase_ab | iso_8601_milliseconds | datetime.timestamp.iso_8601_milliseconds | categorical |
| coverage_closure_phase_ab | sedol | finance.securities.sedol | alphanumeric_id |
| coverage_closure_phase_ab | si_number | representation.numeric.si_number | file_size |
| coverage_closure_phase_ab | word | representation.text.word | word |
| datetime_coverage | fiscal_year | fiscal year | year |

## Regressions

| Dataset | Column | expected | v18_label | confidence |
|---|---|---|---|---|
| codes_and_ids | sha256 | hash | tsid | 0.90 |
| coverage_closure_phase_ab | mdy_short_slash | mdy_short_slash | dmy_short_slash | 0.67 |
| coverage_closure_phase_ab | token_urlsafe | token_urlsafe | url | 0.50 |
| coverage_closure_phase_ab | weekday_full_month | weekday_full_month | weekday_abbreviated_month | 0.85 |
| new_representation | inchi | inchi | dmy_short_slash | 0.91 |
| new_representation | smiles | smiles | entity_name | 0.49 |
| new_technology | git_sha | hash | tsid | 0.82 |
| tech_systems | server_hostname | hostname | email | 0.54 |

## Persistent misses — same prediction (beyond-retrain levers)

Count: 44. These failures did not respond to the v18 retrain; they require a beyond-retrain lever (per-type generator, Sharpen rule, or taxonomy edit).

| Dataset | Column | gt_label | shared prediction |
|---|---|---|---|
| coverage_closure_phase_ab | amount_accounting | finance.currency.amount_accounting | amount |
| coverage_closure_phase_ab | amount_apostrophe | finance.currency.amount_apostrophe | amount |
| coverage_closure_phase_ab | amount_code_prefix | finance.currency.amount_code_prefix | amount |
| coverage_closure_phase_ab | amount_comma | finance.currency.amount_comma | amount |
| coverage_closure_phase_ab | amount_comma_suffix | finance.currency.amount_comma_suffix | amount |
| coverage_closure_phase_ab | amount_crypto | finance.currency.amount_crypto | amount |
| coverage_closure_phase_ab | amount_lakh | finance.currency.amount_lakh | amount |
| coverage_closure_phase_ab | amount_multisym | finance.currency.amount_multisym | amount |
| coverage_closure_phase_ab | amount_neg_trailing | finance.currency.amount_neg_trailing | amount |
| coverage_closure_phase_ab | amount_nodecimal | finance.currency.amount_nodecimal | amount |
| coverage_closure_phase_ab | amount_space | finance.currency.amount_space | amount |
| coverage_closure_phase_ab | calling_code | geography.contact.calling_code | plain_text |
| coverage_closure_phase_ab | csv | container.object.csv | categorical |
| coverage_closure_phase_ab | discrete_ordinal | representation.discrete.ordinal | categorical |
| coverage_closure_phase_ab | ethereum_address | finance.crypto.ethereum_address | full_address |
| coverage_closure_phase_ab | excel_format | representation.file.excel_format | word |
| coverage_closure_phase_ab | gender_code | identity.person.gender_code | categorical |
| coverage_closure_phase_ab | html | container.object.html | categorical |
| coverage_closure_phase_ab | iso_microseconds | datetime.timestamp.iso_microseconds | sql_microseconds |
| coverage_closure_phase_ab | jp_era_short | datetime.date.jp_era_short | alphanumeric_id |
| coverage_closure_phase_ab | json_array | container.object.json_array | categorical |
| coverage_closure_phase_ab | julian | datetime.date.julian | integer_number |
| coverage_closure_phase_ab | numeric_code | representation.identifier.numeric_code | integer_number |
| coverage_closure_phase_ab | ordinal | datetime.date.ordinal | abbreviated_month |
| coverage_closure_phase_ab | password | identity.person.password | password |
| coverage_closure_phase_ab | plain_text | representation.text.plain_text | categorical |
| coverage_closure_phase_ab | query_string | container.key_value.query_string | categorical |
| coverage_closure_phase_ab | semicolon_separated | container.array.semicolon_separated | categorical |
| coverage_closure_phase_ab | short_dmy | datetime.date.short_dmy | dmy_slash |
| coverage_closure_phase_ab | short_mdy | datetime.date.short_mdy | mdy_slash |
| coverage_closure_phase_ab | short_ymd | datetime.date.short_ymd | ymd_slash |
| coverage_closure_phase_ab | state_code | geography.location.state_code | region |
| coverage_closure_phase_ab | street_name | geography.address.street_name | full_name |
| coverage_closure_phase_ab | street_suffix | geography.address.street_suffix | street_address |
| coverage_closure_phase_ab | unix_microseconds | datetime.epoch.unix_microseconds | unix_seconds |
| coverage_closure_phase_ab | whitespace_separated | container.array.whitespace_separated | entity_name |
| coverage_closure_phase_ab | xml | container.object.xml | categorical |
| coverage_closure_phase_ab | yaml | container.object.yaml | categorical |
| coverage_closure_phase_ab | yield | finance.rate.yield | percentage |
| earthquakes_2024 | id | alphanumeric id | username |
| multilingual | locale | language code | word |
| new_geography | geojson | geojson | plain_text |
| people_directory | phone | telephone | ssn |
| tech_systems | user_agent | user agent | jwt |

## Persistent misses — churn (both wrong, different predictions)

| Dataset | Column | gt_label | v16_label | v18_label |
|---|---|---|---|---|
| coverage_closure_phase_ab | measurement_unit | representation.scientific.measurement_unit | entity_name | gender_code |
| coverage_closure_phase_ab | pg_short_offset | datetime.timestamp.pg_short_offset | categorical | rfc_3339 |
| network_logs | user_agent | user agent | docker_ref | whitespace_separated |
