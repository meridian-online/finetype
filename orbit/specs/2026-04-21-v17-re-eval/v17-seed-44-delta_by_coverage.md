# Eval Delta by Coverage Origin

Split of the expanded-eval profile-evaluation results by coverage origin: pre-existing eval rows vs rows added under Phase A+B coverage closure (ac-05).

**Generated from** `eval/eval_output/predictions.json` via `scripts/eval_delta_by_coverage.py`.

**Coverage marker:** manifest `file_path` contains `coverage_closure_phase_ab` → newly_covered.

## Subset Scores

| Bucket | Columns | Label matches | Accuracy |
|---|---|---|---|
| previously_covered | 338 | 248 | 73.4% |
| newly_covered | 110 | 54 | 49.1% |
| **combined** | **448** | **302** | **67.4%** |

## Newly-covered per-type delta

Every closure row scored against v16. A miss here is expected and diagnostic — v16 was never trained against these types.

| Dataset | Column | gt_label | predicted | match | confidence |
|---|---|---|---|---|---|
| coverage_closure_phase_ab | comma_separated | container.array.comma_separated | representation.text.entity_name | ❌ | 0.45 |
| coverage_closure_phase_ab | pipe_separated | container.array.pipe_separated | representation.discrete.categorical | ❌ | 0.61 |
| coverage_closure_phase_ab | semicolon_separated | container.array.semicolon_separated | representation.discrete.categorical | ❌ | 0.47 |
| coverage_closure_phase_ab | whitespace_separated | container.array.whitespace_separated | representation.text.entity_name | ❌ | 0.55 |
| coverage_closure_phase_ab | query_string | container.key_value.query_string | representation.discrete.categorical | ❌ | 0.90 |
| coverage_closure_phase_ab | csv | container.object.csv | representation.discrete.categorical | ❌ | 0.87 |
| coverage_closure_phase_ab | html | container.object.html | representation.discrete.categorical | ❌ | 0.94 |
| coverage_closure_phase_ab | json_array | container.object.json_array | representation.discrete.categorical | ❌ | 0.97 |
| coverage_closure_phase_ab | xml | container.object.xml | representation.discrete.categorical | ❌ | 0.95 |
| coverage_closure_phase_ab | yaml | container.object.yaml | representation.discrete.categorical | ❌ | 0.88 |
| coverage_closure_phase_ab | periodicity | datetime.component.periodicity | datetime.component.periodicity | ✅ | 1.00 |
| coverage_closure_phase_ab | abbrev_month_no_comma | datetime.date.abbrev_month_no_comma | datetime.date.abbrev_month_no_comma | ✅ | 1.00 |
| coverage_closure_phase_ab | chinese_ymd | datetime.date.chinese_ymd | datetime.date.chinese_ymd | ✅ | 0.99 |
| coverage_closure_phase_ab | compact_mdy | datetime.date.compact_mdy | datetime.date.compact_mdy | ✅ | 1.00 |
| coverage_closure_phase_ab | compact_ym | datetime.date.compact_ym | datetime.date.compact_ym | ✅ | 1.00 |
| coverage_closure_phase_ab | dmy_dash_abbrev | datetime.date.dmy_dash_abbrev | datetime.date.dmy_dash_abbrev | ✅ | 1.00 |
| coverage_closure_phase_ab | dmy_dash_abbrev_short | datetime.date.dmy_dash_abbrev_short | datetime.date.dmy_dash_abbrev_short | ✅ | 0.99 |
| coverage_closure_phase_ab | dmy_short_dot | datetime.date.dmy_short_dot | datetime.date.dmy_short_dot | ✅ | 0.97 |
| coverage_closure_phase_ab | dmy_short_slash | datetime.date.dmy_short_slash | datetime.date.dmy_short_slash | ✅ | 0.99 |
| coverage_closure_phase_ab | dmy_space_abbrev | datetime.date.dmy_space_abbrev | datetime.date.dmy_space_abbrev | ✅ | 1.00 |
| coverage_closure_phase_ab | dmy_space_full | datetime.date.dmy_space_full | datetime.date.dmy_space_full | ✅ | 1.00 |
| coverage_closure_phase_ab | full_month_no_comma | datetime.date.full_month_no_comma | datetime.date.full_month_no_comma | ✅ | 1.00 |
| coverage_closure_phase_ab | jp_era_long | datetime.date.jp_era_long | datetime.date.jp_era_long | ✅ | 1.00 |
| coverage_closure_phase_ab | jp_era_short | datetime.date.jp_era_short | representation.identifier.alphanumeric_id | ❌ | 0.92 |
| coverage_closure_phase_ab | julian | datetime.date.julian | representation.numeric.integer_number | ❌ | 0.70 |
| coverage_closure_phase_ab | korean_ymd | datetime.date.korean_ymd | datetime.date.korean_ymd | ✅ | 1.00 |
| coverage_closure_phase_ab | mdy_short_slash | datetime.date.mdy_short_slash | datetime.date.dmy_short_slash | ❌ | 0.54 |
| coverage_closure_phase_ab | month_year_abbrev | datetime.date.month_year_abbrev | datetime.date.month_year_abbrev | ✅ | 1.00 |
| coverage_closure_phase_ab | month_year_full | datetime.date.month_year_full | datetime.date.month_year_full | ✅ | 0.99 |
| coverage_closure_phase_ab | month_year_slash | datetime.date.month_year_slash | datetime.component.year | ❌ | 0.50 |
| coverage_closure_phase_ab | ordinal | datetime.date.ordinal | datetime.date.abbreviated_month | ❌ | 0.60 |
| coverage_closure_phase_ab | short_dmy | datetime.date.short_dmy | datetime.date.dmy_slash | ❌ | 0.90 |
| coverage_closure_phase_ab | short_mdy | datetime.date.short_mdy | datetime.date.mdy_slash | ❌ | 0.95 |
| coverage_closure_phase_ab | short_ymd | datetime.date.short_ymd | datetime.date.ymd_slash | ❌ | 0.85 |
| coverage_closure_phase_ab | weekday_abbreviated_month | datetime.date.weekday_abbreviated_month | datetime.date.weekday_abbreviated_month | ✅ | 1.00 |
| coverage_closure_phase_ab | weekday_dmy_full | datetime.date.weekday_dmy_full | datetime.date.weekday_dmy_full | ✅ | 0.62 |
| coverage_closure_phase_ab | weekday_full_month | datetime.date.weekday_full_month | datetime.date.weekday_abbreviated_month | ❌ | 0.87 |
| coverage_closure_phase_ab | year_month | datetime.date.year_month | datetime.date.year_month | ✅ | 0.97 |
| coverage_closure_phase_ab | ymd_slash | datetime.date.ymd_slash | datetime.date.ymd_slash | ✅ | 1.00 |
| coverage_closure_phase_ab | unix_microseconds | datetime.epoch.unix_microseconds | datetime.epoch.unix_seconds | ❌ | 0.99 |
| coverage_closure_phase_ab | unix_milliseconds | datetime.epoch.unix_milliseconds | datetime.epoch.unix_milliseconds | ✅ | 1.00 |
| coverage_closure_phase_ab | unix_seconds | datetime.epoch.unix_seconds | datetime.epoch.unix_seconds | ✅ | 0.73 |
| coverage_closure_phase_ab | hm_24h | datetime.time.hm_24h | datetime.time.hm_24h | ✅ | 0.96 |
| coverage_closure_phase_ab | hms_24h | datetime.time.hms_24h | datetime.time.hms_24h | ✅ | 1.00 |
| coverage_closure_phase_ab | iso | datetime.time.iso | datetime.time.hms_24h | ❌ | 1.00 |
| coverage_closure_phase_ab | ctime | datetime.timestamp.ctime | datetime.timestamp.ctime | ✅ | 0.97 |
| coverage_closure_phase_ab | dot_dmy_24h | datetime.timestamp.dot_dmy_24h | datetime.timestamp.dot_dmy_24h | ✅ | 0.91 |
| coverage_closure_phase_ab | dot_ymd_24h | datetime.timestamp.dot_ymd_24h | datetime.timestamp.dot_ymd_24h | ✅ | 0.98 |
| coverage_closure_phase_ab | epoch_nanoseconds | datetime.timestamp.epoch_nanoseconds | datetime.timestamp.epoch_nanoseconds | ✅ | 1.00 |
| coverage_closure_phase_ab | iso_8601_compact | datetime.timestamp.iso_8601_compact | datetime.timestamp.iso_8601_compact | ✅ | 0.64 |
| coverage_closure_phase_ab | iso_8601_micros_offset | datetime.timestamp.iso_8601_micros_offset | datetime.timestamp.iso_8601_micros_offset | ✅ | 1.00 |
| coverage_closure_phase_ab | iso_8601_microseconds | datetime.timestamp.iso_8601_microseconds | datetime.timestamp.iso_microseconds | ❌ | 1.00 |
| coverage_closure_phase_ab | iso_8601_millis_offset | datetime.timestamp.iso_8601_millis_offset | datetime.timestamp.iso_8601_millis_offset | ✅ | 1.00 |
| coverage_closure_phase_ab | iso_8601_milliseconds | datetime.timestamp.iso_8601_milliseconds | datetime.timestamp.iso_microseconds | ❌ | 0.58 |
| coverage_closure_phase_ab | iso_microseconds | datetime.timestamp.iso_microseconds | datetime.timestamp.sql_microseconds | ❌ | 1.00 |
| coverage_closure_phase_ab | iso_space_zulu | datetime.timestamp.iso_space_zulu | datetime.timestamp.iso_space_zulu | ✅ | 1.00 |
| coverage_closure_phase_ab | mdy_24h | datetime.timestamp.mdy_24h | datetime.timestamp.mdy_24h | ✅ | 1.00 |
| coverage_closure_phase_ab | pg_short_offset | datetime.timestamp.pg_short_offset | representation.discrete.categorical | ❌ | 0.44 |
| coverage_closure_phase_ab | rfc_2822_ordinal | datetime.timestamp.rfc_2822_ordinal | datetime.timestamp.rfc_2822 | ❌ | 0.85 |
| coverage_closure_phase_ab | slash_ymd_24h | datetime.timestamp.slash_ymd_24h | datetime.timestamp.slash_ymd_24h | ✅ | 1.00 |
| coverage_closure_phase_ab | sql_microseconds | datetime.timestamp.sql_microseconds | datetime.timestamp.sql_microseconds | ✅ | 1.00 |
| coverage_closure_phase_ab | sql_microseconds_offset | datetime.timestamp.sql_microseconds_offset | datetime.timestamp.sql_microseconds_offset | ✅ | 1.00 |
| coverage_closure_phase_ab | sql_milliseconds | datetime.timestamp.sql_milliseconds | datetime.timestamp.sql_milliseconds | ✅ | 1.00 |
| coverage_closure_phase_ab | ethereum_address | finance.crypto.ethereum_address | geography.address.full_address | ❌ | 1.00 |
| coverage_closure_phase_ab | amount | finance.currency.amount | finance.currency.amount | ✅ | 1.00 |
| coverage_closure_phase_ab | amount_accounting | finance.currency.amount_accounting | finance.currency.amount | ❌ | 0.55 |
| coverage_closure_phase_ab | amount_apostrophe | finance.currency.amount_apostrophe | finance.currency.amount | ❌ | 0.73 |
| coverage_closure_phase_ab | amount_code_prefix | finance.currency.amount_code_prefix | finance.currency.amount | ❌ | 0.99 |
| coverage_closure_phase_ab | amount_comma | finance.currency.amount_comma | finance.currency.amount | ❌ | 0.94 |
| coverage_closure_phase_ab | amount_comma_suffix | finance.currency.amount_comma_suffix | finance.currency.amount | ❌ | 0.96 |
| coverage_closure_phase_ab | amount_crypto | finance.currency.amount_crypto | finance.currency.amount | ❌ | 0.64 |
| coverage_closure_phase_ab | amount_lakh | finance.currency.amount_lakh | finance.currency.amount | ❌ | 0.86 |
| coverage_closure_phase_ab | amount_multisym | finance.currency.amount_multisym | finance.currency.amount | ❌ | 0.50 |
| coverage_closure_phase_ab | amount_neg_trailing | finance.currency.amount_neg_trailing | finance.currency.amount | ❌ | 0.83 |
| coverage_closure_phase_ab | amount_nodecimal | finance.currency.amount_nodecimal | finance.currency.amount | ❌ | 1.00 |
| coverage_closure_phase_ab | amount_space | finance.currency.amount_space | finance.currency.amount | ❌ | 0.70 |
| coverage_closure_phase_ab | basis_points | finance.rate.basis_points | finance.rate.basis_points | ✅ | 0.57 |
| coverage_closure_phase_ab | yield | finance.rate.yield | representation.numeric.percentage | ❌ | 1.00 |
| coverage_closure_phase_ab | sedol | finance.securities.sedol | representation.discrete.categorical | ❌ | 0.45 |
| coverage_closure_phase_ab | street_name | geography.address.street_name | identity.person.full_name | ❌ | 0.97 |
| coverage_closure_phase_ab | street_suffix | geography.address.street_suffix | geography.address.street_address | ❌ | 0.98 |
| coverage_closure_phase_ab | calling_code | geography.contact.calling_code | representation.text.plain_text | ❌ | 0.95 |
| coverage_closure_phase_ab | continent | geography.location.continent | geography.location.continent | ✅ | 1.00 |
| coverage_closure_phase_ab | state_code | geography.location.state_code | geography.location.region | ❌ | 0.94 |
| coverage_closure_phase_ab | dea_number | identity.medical.dea_number | identity.medical.dea_number | ✅ | 1.00 |
| coverage_closure_phase_ab | ndc | identity.medical.ndc | identity.medical.ndc | ✅ | 1.00 |
| coverage_closure_phase_ab | blood_type | identity.person.blood_type | identity.person.blood_type | ✅ | 0.99 |
| coverage_closure_phase_ab | gender_code | identity.person.gender_code | representation.discrete.categorical | ❌ | 0.71 |
| coverage_closure_phase_ab | password | identity.person.password | identity.credential.password | ❌ | 0.51 |
| coverage_closure_phase_ab | initials | representation.boolean.initials | representation.boolean.initials | ✅ | 0.97 |
| coverage_closure_phase_ab | terms | representation.boolean.terms | representation.boolean.terms | ✅ | 1.00 |
| coverage_closure_phase_ab | discrete_ordinal | representation.discrete.ordinal | representation.discrete.categorical | ❌ | 0.87 |
| coverage_closure_phase_ab | excel_format | representation.file.excel_format | representation.discrete.categorical | ❌ | 0.52 |
| coverage_closure_phase_ab | extension | representation.file.extension | representation.file.extension | ✅ | 1.00 |
| coverage_closure_phase_ab | color_rgb | representation.format.color_rgb | representation.format.color_rgb | ✅ | 1.00 |
| coverage_closure_phase_ab | numeric_code | representation.identifier.numeric_code | representation.numeric.integer_number | ❌ | 0.79 |
| coverage_closure_phase_ab | si_number | representation.numeric.si_number | representation.numeric.si_number | ✅ | 0.81 |
| coverage_closure_phase_ab | dna_sequence | representation.scientific.dna_sequence | representation.identifier.alphanumeric_id | ❌ | 0.39 |
| coverage_closure_phase_ab | measurement_unit | representation.scientific.measurement_unit | representation.text.entity_name | ❌ | 0.42 |
| coverage_closure_phase_ab | protein_sequence | representation.scientific.protein_sequence | representation.identifier.alphanumeric_id | ❌ | 0.35 |
| coverage_closure_phase_ab | rna_sequence | representation.scientific.rna_sequence | representation.scientific.rna_sequence | ✅ | 0.58 |
| coverage_closure_phase_ab | emoji | representation.text.emoji | representation.text.emoji | ✅ | 1.00 |
| coverage_closure_phase_ab | plain_text | representation.text.plain_text | representation.discrete.categorical | ❌ | 0.97 |
| coverage_closure_phase_ab | word | representation.text.word | representation.discrete.categorical | ❌ | 0.87 |
| coverage_closure_phase_ab | doi | technology.code.doi | technology.code.doi | ✅ | 1.00 |
| coverage_closure_phase_ab | imei | technology.code.imei | technology.code.imei | ✅ | 1.00 |
| coverage_closure_phase_ab | token_urlsafe | technology.cryptographic.token_urlsafe | technology.internet.url | ❌ | 0.50 |
| coverage_closure_phase_ab | http_method | technology.internet.http_method | representation.discrete.categorical | ❌ | 0.34 |
| coverage_closure_phase_ab | ip_v4_with_port | technology.internet.ip_v4_with_port | technology.internet.ip_v4_with_port | ✅ | 1.00 |
| coverage_closure_phase_ab | top_level_domain | technology.internet.top_level_domain | technology.internet.top_level_domain | ✅ | 0.95 |

## Diagnostic-only note

Per ac-12 and MADR 0054, this re-score is NOT a promotion baseline. v16 shipped against the pre-closure 242-column eval at 235/242 (97.1%). The drop to the combined 84.4% shown here reflects v16's untrained blind spots that the coverage closure now surfaces — which is exactly what ac-12 exists to measure.
