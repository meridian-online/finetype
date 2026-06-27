# ac-0 — Model label-space reshape: CEDE / KEEP / EXCLUDE partition

Spec `2026-06-27-model-label-space-reshape`. Gate AC. Source: post-challenge classification of all 244 model-predicted leaves + the shipping recovery-rule inventory.

> **Provenance:** workflow `wf_9a28f0ad-47a` (16 agents — 7 domain classifiers → recovery audit → 7 adversarial challengers → synthesis). Machine-readable cede sets in `ac0_partition_sets.json`.
>
> **Author verification (2026-06-27):** the single most load-bearing recovery claim — that the ~76 *delimited* datetime leaves recover value-only while the 8 *bare-number* epoch/compact leaves do not — was checked directly against `datetime_format_refinement` (mod.rs:2338): `if !detected.delimited && !result.label.starts_with("datetime.") { return; }`. Confirmed: delimited formats fire regardless of the model's label; bare-number formats need the model's `datetime.*` anchor the reshape removes. The partition's hold-back of the 8 bare-number leaves is correct.

## 1. Headline

**134 of the 244 leaves are types the engine can name from the values alone — dates, UUIDs, IPs, emails, ISO timestamps — so we can stop asking the model to learn them and let a deterministic recogniser own them, shrinking the model from 244 classes to 110 and freeing its capacity for the genuinely context-dependent calls (city vs region, integer vs id) that only a model can make.** Aggressive ceiling is 81 classes (cede everything value-recognisable), but 29 of those leaves have no recogniser yet and ceding them today loses recall — hold them.

## 2. The partition

Four buckets over 244 leaves: **CEDE_CLEAN 125, CEDE_GATED 38, EXCLUDE 23, KEEP 58.** CEDE = a deterministic recogniser can (or could) own it, so the model need not carry the class. KEEP = no value-only validator can own it; the model's contextual judgement is load-bearing. EXCLUDE = open-vocabulary or broad-numeric leaf where ceding strictly loses recall with zero recovery path.

### 2a. CEDE_CLEAN (125) — value-self-sufficient, no header needed

Sorted by domain. `recovery`: `SHIPS` = a value-only recovery rule already ships; `GENERIC` = value-self-sufficient, recoverable by the proposed generic closed-validator rule (nothing recovers it *today*).

| leaf | validator | recovery | why value-conclusive |
|---|---|---|---|
| container.key_value.query_string | format_exclusive | GENERIC | `&`-joined `k=v` signature distinctive |
| container.object.html | closed_enum | GENERIC | closed HTML-tag-name enum, not permissive |
| container.object.json | format_exclusive | GENERIC | strict PARSE_JSON is checksum-grade |
| container.object.json_array | format_exclusive | GENERIC | bracket-wrapped array parse conclusive |
| container.object.xml | format_exclusive | GENERIC | strict XML parse; random data won't parse |
| datetime.component.day_of_week | closed_enum | GENERIC | closed weekday enum incl. locales |
| datetime.component.month_name | closed_enum | GENERIC | closed 12-name enum |
| datetime.component.periodicity | closed_enum | GENERIC | tiny closed vocab (Once/Daily/Never) |
| datetime.date.abbrev_month_no_comma | format_exclusive | GENERIC | named-month token closed-set anchors shape |
| datetime.date.abbreviated_month | format_exclusive | GENERIC | named-month delimited date |
| datetime.date.chinese_ymd | format_exclusive | GENERIC | CJK 年/月/日 delimiters unmistakable |
| datetime.date.dmy_dash | format_exclusive | SHIPS | dash date, 4-digit year, in detector |
| datetime.date.dmy_dash_abbrev | format_exclusive | GENERIC | Oracle DD-MON-YYYY, named month |
| datetime.date.dmy_dash_abbrev_short | format_exclusive | GENERIC | named-month dash date |
| datetime.date.dmy_dot | format_exclusive | GENERIC | dotted DMY, 4-digit year |
| datetime.date.dmy_short_dot | format_exclusive | GENERIC | dotted 2-digit triple, date-canonical |
| datetime.date.dmy_short_slash | format_exclusive | GENERIC | slashed 2-digit triple |
| datetime.date.dmy_slash | format_exclusive | SHIPS | slash date, 4-digit year, in detector |
| datetime.date.dmy_space_abbrev | format_exclusive | GENERIC | RFC2822 named-month space date |
| datetime.date.dmy_space_full | format_exclusive | GENERIC | full-month space date |
| datetime.date.full_month_no_comma | format_exclusive | GENERIC | full-month day year |
| datetime.date.iso | format_exclusive | SHIPS | canonical ISO date, in detector |
| datetime.date.iso_week | format_exclusive | GENERIC | literal `W` token (2024-W36) |
| datetime.date.jp_era_long | format_exclusive | GENERIC | kanji era + 年月日 |
| datetime.date.jp_era_short | format_exclusive | GENERIC | [RHSTM] era prefix + slashed date |
| datetime.date.korean_ymd | format_exclusive | GENERIC | Korean 년/월/일 delimiters |
| datetime.date.long_full_month | format_exclusive | GENERIC | full named-month date |
| datetime.date.mdy_dash | format_exclusive | SHIPS | dash date, 4-digit year, in detector |
| datetime.date.mdy_short_slash | format_exclusive | GENERIC | slashed 2-digit triple |
| datetime.date.mdy_slash | format_exclusive | SHIPS | slash date, 4-digit year, in detector |
| datetime.date.month_year_abbrev | format_exclusive | GENERIC | abbrev-month + year |
| datetime.date.month_year_full | format_exclusive | GENERIC | full-month + year |
| datetime.date.month_year_slash | format_exclusive | GENERIC | month-range-constrained MM/YY|YYYY |
| datetime.date.short_dmy | format_exclusive | GENERIC | dashed 2-digit triple, date-canonical |
| datetime.date.short_mdy | format_exclusive | GENERIC | dashed 2-digit triple |
| datetime.date.short_ymd | format_exclusive | GENERIC | dashed 2-digit triple, year-first |
| datetime.date.weekday_abbreviated_month | format_exclusive | GENERIC | weekday + named month + year |
| datetime.date.weekday_dmy_full | format_exclusive | GENERIC | named weekday + month |
| datetime.date.weekday_full_month | format_exclusive | GENERIC | weekday + full month + year |
| datetime.date.year_month | format_exclusive | SHIPS | `\d{4}-\d{2}` in detector (dash + 12-month) |
| datetime.date.ymd_dot | format_exclusive | SHIPS | dot YMD year-first, in detector |
| datetime.date.ymd_slash | format_exclusive | SHIPS | slash YMD year-first, in detector |
| datetime.duration.iso_8601 | format_exclusive | GENERIC | leading `P` ISO-duration grammar |
| datetime.offset.utc | format_exclusive | GENERIC | literal `UTC ` prefix; bare offsets vetoed |
| datetime.period.fiscal_year | format_exclusive | GENERIC | literal `FY` prefix |
| datetime.period.quarter | format_exclusive | GENERIC | literal `Q` + quarter digit + year |
| datetime.time.hm_12h | format_exclusive | GENERIC | colon time + AM/PM |
| datetime.time.hm_24h | format_exclusive | SHIPS | colon HH:MM, in detector |
| datetime.time.hms_12h | format_exclusive | GENERIC | HH:MM:SS + AM/PM |
| datetime.time.hms_24h | format_exclusive | SHIPS | colon 24h time, in detector |
| datetime.time.iso | format_exclusive | SHIPS | colon time + fractional seconds, in detector |
| datetime.timestamp.clf | format_exclusive | GENERIC | Apache CLF bracketed date:time+offset |
| datetime.timestamp.ctime | format_exclusive | GENERIC | weekday+month+time+year |
| datetime.timestamp.dmy_hm | format_exclusive | GENERIC | slashed date + colon time |
| datetime.timestamp.dot_dmy_24h | format_exclusive | GENERIC | dot date + colon time |
| datetime.timestamp.dot_ymd_24h | format_exclusive | SHIPS | dot YMD + colon time, in detector |
| datetime.timestamp.iso_8601 | format_exclusive | SHIPS | T + Z canonical, in detector |
| datetime.timestamp.iso_8601_compact | format_exclusive | GENERIC | literal T between 8 and 6 digits |
| datetime.timestamp.iso_8601_micros_offset | format_exclusive | SHIPS | T + micros + offset, in detector |
| datetime.timestamp.iso_8601_microseconds | format_exclusive | SHIPS | T + micros + Z, in detector |
| datetime.timestamp.iso_8601_millis_offset | format_exclusive | SHIPS | T + millis + offset, in detector |
| datetime.timestamp.iso_8601_milliseconds | format_exclusive | SHIPS | toISOString shape, in detector |
| datetime.timestamp.iso_8601_offset | format_exclusive | SHIPS | T + numeric offset, in detector |
| datetime.timestamp.iso_microseconds | format_exclusive | SHIPS | T + micros zoneless, in detector |
| datetime.timestamp.iso_milliseconds | format_exclusive | SHIPS | T + millis zoneless, in detector |
| datetime.timestamp.iso_seconds | format_exclusive | SHIPS | T zoneless date+time, in detector |
| datetime.timestamp.iso_space_zulu | format_exclusive | SHIPS | space separator + Z, in detector |
| datetime.timestamp.mdy_12h | format_exclusive | GENERIC | slashed date + AM/PM time |
| datetime.timestamp.mdy_24h | format_exclusive | GENERIC | slashed date + colon time |
| datetime.timestamp.pg_short_offset | format_exclusive | GENERIC | SQL datetime + micros + 2-digit offset |
| datetime.timestamp.rfc_2822 | format_exclusive | GENERIC | email-header timestamp shape |
| datetime.timestamp.rfc_2822_ordinal | format_exclusive | GENERIC | ordinal suffix + month abbrev + offset |
| datetime.timestamp.rfc_3339 | format_exclusive | SHIPS | space separator + offset/Z, in detector |
| datetime.timestamp.slash_ymd_24h | format_exclusive | SHIPS | slash YMD + colon time, in detector |
| datetime.timestamp.sql_microseconds | format_exclusive | SHIPS | SQL datetime + micros, in detector |
| datetime.timestamp.sql_microseconds_offset | format_exclusive | SHIPS | pg timestamptz, in detector |
| datetime.timestamp.sql_milliseconds | format_exclusive | SHIPS | SQL datetime + millis, in detector |
| datetime.timestamp.sql_standard | format_exclusive | SHIPS | space SQL datetime, main detector target |
| datetime.timestamp.syslog_bsd | format_exclusive | GENERIC | month abbrev + padded day + time |
| finance.banking.iban | regex_strong | GENERIC | 2-alpha+2-digit prefix over 15-34 alnum, no lookalike |
| finance.crypto.bitcoin_address | regex_strong | GENERIC | Base58Check 1/3 prefix or bc1 bech32 |
| finance.crypto.ethereum_address | regex_strong | GENERIC | `0x` + exactly 40 hex |
| finance.currency.currency_code | closed_enum | GENERIC | closed ISO-4217 enum; word collisions wash out at column |
| finance.currency.currency_symbol | format_exclusive | GENERIC | all-chars in closed Unicode `Sc` category |
| finance.rate.basis_points | regex_strong | GENERIC | mandatory `bps`/`bp` unit suffix |
| finance.securities.figi | regex_strong | GENERIC | mandatory `G` at pos 3 + consonant-only alpha |
| geography.address.street_suffix | closed_enum | GENERIC | closed St/Ave/Blvd/Rd enum |
| geography.coordinate.dms | format_exclusive | GENERIC | deg/min/sec glyphs + N/S/E/W |
| geography.coordinate.mgrs | regex_strong | GENERIC | band letter C-X + all-digit tail |
| geography.coordinate.plus_code | format_exclusive | GENERIC | literal `+` + 20-symbol OLC alphabet |
| geography.format.wkt | regex_strong | GENERIC | leading OGC keyword (POINT/POLYGON) |
| geography.location.continent | closed_enum | GENERIC | closed 7-value enum |
| geography.transportation.iso6346 | regex_strong | GENERIC | pos-4 U/J/Z + 7-digit tail (checksum claim overstated; regex carries it) |
| identity.commerce.isrc | format_exclusive | GENERIC | fixed 2-alpha+3-alnum+7-digit 12-char |
| identity.government.pan_india | format_exclusive | GENERIC | AAAAA9999A, no common lookalike |
| identity.government.vin | format_exclusive | GENERIC | exactly-17 alnum excl I/O/Q |
| identity.medical.icd10 | format_exclusive | GENERIC | letter+digit+[0-9AB] + dotted suffix |
| identity.person.email | regex_strong | GENERIC | RFC-5322 local@domain envelope |
| identity.person.email_display | regex_strong | GENERIC | Name &lt;local@domain&gt; envelope |
| identity.person.gender | closed_enum | GENERIC | closed {male,female,other,unknown} |
| identity.person.phone_e164 | regex_strong | GENERIC | mandatory `+` then 7-15 digits |
| representation.boolean.terms | closed_enum | GENERIC | closed boolean-word enum (true/false/yes/no) |
| representation.file.mime_type | format_exclusive | GENERIC | mandatory slash + closed top-level prefix |
| representation.format.color_hex | regex_strong | GENERIC | (demoted CEDE_GATED→recovery via #/header; see audit) |
| representation.format.color_hsl | regex_strong | GENERIC | mandatory `hsl(`/`hsla(` prefix |
| representation.format.color_rgb | regex_strong | GENERIC | (demoted; rgb() prefix optional — see audit) |
| representation.identifier.uuid | regex_strong | GENERIC | 8-4-4-4-12 hex, collision-free |
| representation.numeric.si_number | format_exclusive | GENERIC | mandatory trailing K/M/B/T suffix |
| representation.scientific.inchi | format_exclusive | GENERIC | mandatory `InChI=1` prefix |
| representation.text.emoji | format_exclusive | GENERIC | Unicode symbol-category-only |
| technology.cloud.aws_arn | regex_strong | GENERIC | `arn:(aws...):` prefix + closed partition enum |
| technology.cloud.s3_uri | regex_strong | GENERIC | literal `s3://` scheme |
| technology.code.doi | regex_strong | GENERIC | `10.`+registrant+`/` grammar |
| technology.cryptographic.jwt | regex_strong | GENERIC | 3 dot-separated base64url segments ≥10 |
| technology.filesystem.windows_path | regex_strong | SHIPS | drive-letter/UNC root; structured_string_refinement |
| technology.identifier.ulid | regex_strong | GENERIC | 26-char Crockford Base32, no I/L/O/U |
| technology.internet.cidr | regex_strong | GENERIC | octet-validated IPv4 + /0-32 |
| technology.internet.data_uri | regex_strong | GENERIC | literal `data:` scheme + `,` |
| technology.internet.http_method | closed_enum | GENERIC | closed 27-variant verb enum |
| technology.internet.ip_v4 | regex_strong | GENERIC | octet-range dotted-quad (exemplar) |
| technology.internet.ip_v4_with_port | regex_strong | GENERIC | octet-validated IPv4 + :port |
| technology.internet.ip_v6 | regex_strong | GENERIC | eight colon hex groups |
| technology.internet.mac_address | regex_strong | GENERIC | six hex pairs colon/hyphen (exemplar) |
| technology.internet.message_id | regex_strong | SHIPS | `<...@...>` grammar; structured_string_refinement |
| technology.internet.url | regex_strong | SHIPS | `//` + dotted host; structured_string_refinement |
| technology.internet.urn | regex_strong | GENERIC | literal `urn:` + nid:nss grammar |
| technology.internet.user_agent | regex_strong | GENERIC | distinctive client-string prefix list |

CEDE_CLEAN recovery split: **31 SHIPS, 94 GENERIC.**

### 2b. CEDE_GATED (38) — value-recognisable but needs a gate (header or sibling-precedence) to be safe

| leaf | validator | recovery | gate / reason |
|---|---|---|---|
| datetime.date.compact_dmy | regex_permissive | RECALL_LOSS_RISK | bare \d{8} collides with ints/siblings; needs date-ish header |
| datetime.date.compact_mdy | regex_permissive | RECALL_LOSS_RISK | bare 8-digit, header-gated only |
| datetime.date.compact_ym | regex_permissive | RECALL_LOSS_RISK | 6-digit YYYYMM collides with year/int |
| datetime.date.compact_ymd | regex_permissive | RECALL_LOSS_RISK | bare \d{8} = int/epoch-day |
| datetime.date.ordinal | format_exclusive | GENERIC | \d{4}-\d{3} collides with lot/product codes; context gate |
| datetime.epoch.unix_microseconds | regex_permissive | RECALL_LOSS_RISK | 16-digit = large IDs; **anchor removed by reshape** |
| datetime.epoch.unix_milliseconds | regex_permissive | RECALL_LOSS_RISK | 13-digit = snowflake/IDs; anchor removed |
| datetime.epoch.unix_seconds | regex_permissive | RECALL_LOSS_RISK | 10-digit = epoch/id/phone; anchor removed |
| datetime.timestamp.epoch_nanoseconds | regex_permissive | RECALL_LOSS_RISK | 19-digit = BIGINT; anchor removed |
| finance.banking.aba_routing | checksum | NEEDS_NEW_RULE | mod-10 wired; header-gated checksum-promote feasible |
| finance.banking.bsb | regex_strong | NEEDS_NEW_RULE | no checksum; ###-### shape, AU-header gate |
| finance.banking.swift_bic | regex_strong | NEEDS_NEW_RULE | overlaps alnum_id; BIC/SWIFT header |
| finance.payment.credit_card_number | checksum | NEEDS_NEW_RULE | Luhn unwired; header (Luhn+IIN) mandatory |
| finance.securities.cusip | checksum | NEEDS_NEW_RULE | check digit wired; sibling (sedol/isin) precedence needed |
| finance.securities.isin | checksum | NEEDS_NEW_RULE | Luhn unwired; checksum+corroboration rule |
| finance.securities.lei | checksum | NEEDS_NEW_RULE | mod-97 unwired; matches generic 20-char alnum |
| finance.securities.sedol | checksum | NEEDS_NEW_RULE | all-digit form collides; ~1/10 pass, header |
| geography.location.state_code | closed_enum | SHIPS | header_corroborates_state + closed enum (mod.rs:1752) |
| identity.academic.orcid | format_exclusive | NEEDS_NEW_RULE | conclusive shape; rare 16-digit-card lookalike |
| identity.commerce.ean | regex_permissive | NEEDS_NEW_RULE | mod-10 unwired; gtin/product header |
| identity.commerce.isbn | checksum | SHIPS | isbn_header_recovery (only wired identity checksum) |
| identity.commerce.issn | format_exclusive | NEEDS_NEW_RULE | mod-11 unwired; `issn` header |
| identity.commerce.upc | regex_permissive | NEEDS_NEW_RULE | mod-10 unwired; barcode/upc header |
| identity.government.abn | regex_permissive | NEEDS_NEW_RULE | mod-89 unwired; AU `abn` header |
| identity.medical.dea_number | format_exclusive | GENERIC | check unwired; collides with license IDs, gate on format |
| identity.medical.hcpcs | format_exclusive | NEEDS_NEW_RULE | [A-V]\d{4} collides; procedure header |
| identity.medical.loinc | regex_permissive | NEEDS_NEW_RULE | mod-10 unwired; lab/loinc header |
| identity.medical.ndc | format_exclusive | NEEDS_NEW_RULE | \d{11} branch collides; gate on dashed/drug header |
| identity.medical.npi | regex_permissive | NEEDS_NEW_RULE | Luhn unwired; provider/npi header |
| identity.person.blood_type | closed_enum | NEEDS_NEW_RULE | bare A/B/O collide; gate on ± suffix / header |
| representation.file.file_size | format_exclusive | NEEDS_NEW_RULE | bare `1024` = int; size-header gate |
| representation.format.color_hex | regex_strong | GENERIC | `#` optional → bare 123456 collides; #/header gate |
| representation.format.color_rgb | regex_strong | GENERIC | rgb() optional → bare triples collide; prefix/header gate |
| representation.scientific.dna_sequence | format_exclusive | NEEDS_NEW_RULE | ATGC matches CAT/MAN; length+alphabet+header |
| representation.scientific.rna_sequence | format_exclusive | NEEDS_NEW_RULE | AUGC collides with dna/words; length-discriminating |
| technology.code.qualified_name | regex_strong | SHIPS | structured_string_refinement, residual-scoped (mod.rs:2433) |
| technology.cryptographic.hash | format_exclusive | GENERIC | 40/64/128 clean; 32-hex collides with uuid/md5/tsid |
| technology.development.version | regex_strong | GENERIC | N.N.N collides with dotted date/calver; first-part gate |

CEDE_GATED recovery split: **1 SHIPS_VALUE_ONLY, 2 SHIPS_HEADER_GATED (state_code, isbn), 6 GENERIC, 21 NEEDS_NEW_RULE, 8 RECALL_LOSS_RISK.**

### 2c. EXCLUDE (23) — ceding strictly loses recall, no recovery path

Open-vocabulary or broad-numeric leaves. The model's contextual judgement is the only thing that can own these — they stay, and they are the leaves the reshape *protects* capacity for.

| leaf | validator | reason |
|---|---|---|
| geography.address.full_address | none_open_vocab | length-only 10-500; open free-text |
| geography.address.street_name | none_open_vocab | open street-name vocabulary |
| geography.coordinate.latitude | broad_shape | any small decimal; column-mode vs longitude/decimal (latdec history) |
| geography.coordinate.longitude | broad_shape | any decimal in [-180,180]; column-mode disambiguation |
| geography.location.city | none_open_vocab | open city names; canonical guardrail-1 |
| geography.location.country | none_open_vocab | open multilingual country names |
| geography.location.region | none_open_vocab | open state/province/region names |
| geography.transportation.iata_code | regex_permissive | `[A-Z]{3}` matches THE/USA/CEO; set not enumerated |
| geography.transportation.icao_code | regex_permissive | `[A-Z]{4}` any 4-letter token |
| identity.person.first_name | none_open_vocab | open given names (guardrail-1) |
| identity.person.full_name | none_open_vocab | any letter string (guardrail-1) |
| identity.person.height | regex_permissive | optional unit → bare numbers match |
| identity.person.last_name | none_open_vocab | open surnames (guardrail-1) |
| identity.person.password | none_open_vocab | length-only, no pattern — a non-validation |
| identity.person.phone_number | regex_permissive | permissive digit/separator runs |
| identity.person.username | regex_permissive | any short alnum handle (guardrail-1) |
| identity.person.weight | regex_permissive | optional unit → bare numbers match |
| representation.file.extension | none_open_vocab | any 1-10 alnum token = word |
| representation.identifier.numeric_code | none_open_vocab | `[0-9]+` open code vocab = integer (guardrail-1) |
| representation.scientific.measurement_unit | none_open_vocab | ~30-unit enum but real units open (guardrail-1) |
| representation.text.entity_name | none_open_vocab | open org/product/title names (guardrail-1) |
| technology.internet.hostname | none_open_vocab | matches localhost/apple/data (guardrail-1) |
| technology.internet.top_level_domain | regex_permissive | `[a-z]{2,}` any lowercase word; real TLD list not wired |

### 2d. KEEP (58) — permissive/broad validator, model judgement load-bearing

| leaf | validator | recovery | reason |
|---|---|---|---|
| container.array.comma_separated | broad_shape | n/a | zero-rep branch matches bare "apple" |
| container.array.pipe_separated | broad_shape | n/a | base matches any pipe-free string |
| container.array.semicolon_separated | broad_shape | n/a | accepts ~90% of random input |
| container.array.whitespace_separated | broad_shape | n/a | matches any trimmed text |
| container.object.csv | broad_shape | n/a | identical permissive regex to comma_separated |
| container.object.yaml | regex_permissive | RECALL_LOSS_RISK | `Word: text` matches labelled prose |
| datetime.component.year | broad_shape | n/a | `\d{4}` = any 4-digit integer |
| datetime.date.julian | regex_permissive | RECALL_LOSS_RISK | NN-NNN unbounded day; no header/detector anchor |
| datetime.offset.iana | regex_permissive | SHIPS_HEADER_GATED | matches Sales/Marketing; recovery header-only |
| finance.currency.amount | broad_shape | n/a | 4th alternation matches any bare decimal |
| finance.currency.amount_accounting | regex_permissive | n/a | paren/$ optional → plain comma numbers |
| finance.currency.amount_apostrophe | regex_permissive | n/a | CHF/apostrophe optional → bare 123.45 |
| finance.currency.amount_code_prefix | regex_permissive | n/a | `[A-Z]{3}` not enum-validated |
| finance.currency.amount_comma | regex_permissive | n/a | symbol optional → EU-grouped numbers |
| finance.currency.amount_comma_suffix | regex_permissive | n/a | number + trailing symbol |
| finance.currency.amount_crypto | regex_permissive | n/a | any number + any 2-5 uppercase |
| finance.currency.amount_lakh | regex_permissive | n/a | Indian grouping still a numeric amount |
| finance.currency.amount_multisym | regex_permissive | n/a | anchors symbol then unconstrained number |
| finance.currency.amount_neg_trailing | regex_permissive | n/a | number + -/CR/DR |
| finance.currency.amount_nodecimal | regex_permissive | n/a | symbol optional → comma-grouped ints |
| finance.currency.amount_space | regex_permissive | n/a | symbol optional → space-grouped number |
| finance.rate.yield | regex_strong | n/a | signed percentage = percentage; semantic call |
| geography.address.postal_code | regex_permissive | RECALL_LOSS_RISK | bare 4-6 digits = int; only vetoes (demote) ship |
| geography.contact.calling_code | regex_permissive | RECALL_LOSS_RISK | `\+?\d{1,4}` confirms any small number |
| geography.coordinate.coordinates | regex_permissive | RECALL_LOSS_RISK | any signed-decimal pair; sharpen only disambiguates |
| geography.coordinate.geohash | regex_permissive | RECALL_LOSS_RISK | base32 collides with lowercase alnum/IDs |
| geography.index.h3 | regex_permissive | RECALL_LOSS_RISK | 15-hex only, no bit-structure check |
| geography.location.country_code | closed_enum | RECALL_LOSS_RISK | corroboration can't recover a lone ISO-2 col; overlaps state_code |
| geography.transportation.hs_code | regex_permissive | RECALL_LOSS_RISK | 6-10 digit tariff = int; only demotes |
| geography.transportation.unlocode | regex_permissive | RECALL_LOSS_RISK | any 5-char uppercase alnum; only demotes |
| identity.government.ein | format_exclusive | RECALL_LOSS_RISK | NN-NNNNNNN no check digit; shared tax-id headers |
| identity.government.eu_vat | regex_permissive | NEEDS_NEW_RULE | universal pattern accepts most short uppercase |
| identity.government.ssn | format_exclusive | RECALL_LOSS_RISK | bare-9 collides; no checksum; PII |
| identity.medical.cpt | regex_permissive | RECALL_LOSS_RISK | `\d{5}` collides with postal_code |
| identity.person.gender_code | closed_enum | RECALL_LOSS_RISK | single-char M/F/X collides with boolean/rating |
| representation.boolean.binary | closed_enum | n/a | "0"/"1" ambiguous with int/bit/flag |
| representation.boolean.initials | closed_enum | RECALL_LOSS_RISK | single-char T/F/Y/N collides with grades/gender |
| representation.discrete.ordinal | broad_shape | n/a | hard to distinguish from categorical/int |
| representation.file.excel_format | broad_shape | n/a | includes \w + punctuation, ~90% pass |
| representation.identifier.alphanumeric_id | broad_shape | n/a | residual catch-all attractor |
| representation.identifier.increment | broad_shape | n/a | `[0-9]+` = integer without context |
| representation.numeric.decimal_number | broad_shape | n/a | generic float, permissive-by-design |
| representation.numeric.decimal_number_comma | broad_shape | n/a | EU/US comma-decimal ambiguity |
| representation.numeric.integer_number | broad_shape | n/a | any integer |
| representation.numeric.percentage | regex_permissive | n/a | `%` optional → bare numbers pass |
| representation.numeric.scientific_notation | broad_shape | n/a | decimal validator also accepts sci suffix |
| representation.scientific.cas_number | checksum | NEEDS_NEW_RULE | CAS mod-10 NOT implemented; de-facto = NN-NN-N |
| representation.scientific.protein_sequence | regex_permissive | n/a | 21-letter alphabet confirms most uppercase |
| representation.scientific.smiles | regex_permissive | n/a | single 'O'/'CC' indistinguishable from text |
| representation.text.plain_text | broad_shape | n/a | accepts any string |
| representation.text.word | broad_shape | n/a | any single token, ~90% of short strings |
| technology.code.imei | checksum | NEEDS_NEW_RULE | base `\d{15}` permissive; Luhn unwired + Amex collision |
| technology.code.locale_code | regex_permissive | NEEDS_NEW_RULE | `[a-zA-Z]{2,3}` = language/country/state; BCP47 not wired |
| technology.cryptographic.token_urlsafe | regex_permissive | n/a | any 12-128 char token |
| technology.development.calver | regex_strong | RECALL_LOSS_RISK | 2024.02 = decimal, 2023.12.31 = date |
| technology.development.docker_ref | regex_permissive | n/a | registry/tag optional → bare `nginx` matches |
| technology.identifier.snowflake_id | broad_shape | RECALL_LOSS_RISK | `\d{17,20}` = any BIGINT |
| technology.identifier.tsid | regex_strong | RECALL_LOSS_RISK | `[0-9a-f]{32}` = hash/md5/hyphenless-uuid |

## 3. Recovery ledger (every CEDE leaf)

### 3a. Recovery ships or is value-self-sufficient — SAFE TO CEDE NOW (134)

- **Ships a value-only recovery rule today (32):** the delimited-datetime subset routed through `detect_datetime_format` (the SHIPS rows above — iso, sql_standard, the iso_8601 family, dmy/mdy/ymd slash+dash with 4-digit year, hm_24h/hms_24h/time.iso, year_month, ymd_dot/ymd_slash, dot_ymd_24h, slash_ymd_24h), plus `structured_string_refinement` (windows_path, message_id, url) and the residual-scoped `qualified_name`.
- **Ships header-gated recovery today (2):** state_code (`header_corroborates_state` + closed enum), isbn (`isbn_header_recovery`, the only wired identity checksum).
- **Value-self-sufficient, recoverable by the proposed generic closed-validator rule (100):** every GENERIC row — uuid, iban, vin, mac_address, ip_v4/v6, the colour `hsl`/`hex`/`rgb` set (the latter two via the same #/prefix gate the generic rule applies), jwt, ulid, dea_number, pan_india, icd10, phone_e164, hash, version, the non-detector delimited datetimes, etc. Conclusive and collision-free at column level; nothing recovers them *today* but the generic rule cleanly covers them.

These 134 are the conservative cede set. Ceding them now loses no recall provided the generic closed-validator recovery rule ships alongside (ac-1).

### 3b. NEEDS A NEW RECOVERY RULE before ceding — ac-1 prerequisite (29)

Splits into two distinct risk classes:

**Hold as KEEP — recovery anchor removed by the reshape (8, RECALL_LOSS_RISK).** The four bare-integer epoch leaves and four bare-digit compact dates recover *today* only because `datetime_format_refinement` fires when the model already says `datetime.*` (mod.rs:2338) — the reshape removes exactly that anchor, and these shapes are value-identical to BIGINT IDs. No universal validator exists. Demote to KEEP:
`datetime.epoch.unix_seconds/_milliseconds/_microseconds`, `datetime.timestamp.epoch_nanoseconds`, `datetime.date.compact_ymd/_dmy/_mdy/_ym`.

**Build a checksum/format-promote rule first, then cede (21, NEEDS_NEW_RULE).** Checksum-bearing securities/commerce/medical/banking identifiers + the two sequence types: each has a value-only recovery *in principle* (a checksum or a constrained shape), but siblings collide on the same digit shape, so each needs a hand-written checksum-promote with sibling precedence before ceding is safe:
`finance.banking.aba_routing/bsb/swift_bic`, `finance.payment.credit_card_number`, `finance.securities.cusip/isin/lei/sedol`, `identity.academic.orcid`, `identity.commerce.ean/issn/upc`, `identity.government.abn`, `identity.medical.hcpcs/loinc/ndc/npi`, `identity.person.blood_type`, `representation.file.file_size`, `representation.scientific.dna_sequence/rna_sequence`.

(Note: `eu_vat`, `cas_number`, `imei`, `locale_code` also carry a NEEDS_NEW_RULE recovery tag but were ruled KEEP, not CEDE — their validators are permissive *and* their checksums unwired, so they are not cede candidates this round.)

## 4. The n_classes math

```
Current model label space ................................. 244
CEDE_CLEAN ......................................... 125
CEDE_GATED .......................................... 38
EXCLUDE ............................................. 23  (stay — open-vocab/broad-numeric)
KEEP ................................................ 58  (stay — permissive validator)

Conservative (cede only SHIPS/GENERIC recovery = 134):
  244 − 134 = 110 classes        ← RECOMMENDED into ac-1
Aggressive (cede all 163 CEDE leaves):
  244 − 163 =  81 classes        ← needs 21 new rules + accepts 8 recall losses
```

The 110-class conservative target ships with the generic recovery rule and loses no recall. The 81-class aggressive target buys 29 more removed classes but is gated on building 21 checksum-promote rules and accepts an 8-leaf recall loss on the bare-number epoch/compact-date set — not worth it this round.

## 5. Guardrail audit

**PASS — no open-vocabulary leaf landed in any CEDE bucket.** Programmatic check: zero `none_open_vocab` leaves in CEDE_CLEAN or CEDE_GATED. All twelve named guardrail-1 leaves are accounted for in EXCLUDE: city, entity_name, username, numeric_code, full_name, first_name, last_name, region, country, street_name, password, hostname, measurement_unit — every one is EXCLUDE. (latitude/longitude also held in EXCLUDE on the latdec-relocation precedent.)

**Leaves demoted during the challenge (CLEAN→GATED, or →KEEP), with reason:**

- `representation.format.color_hex` — CLEAN→GATED: the `#` is optional, so bare `123456`/`255` collide with integers/hex-IDs; recovery must be #- or header-gated.
- `representation.format.color_rgb` — CLEAN→GATED: `rgb()` prefix optional and channels unbounded to 999, so bare `255,128,0` triples collide with generic numeric triples (same optional-marker flaw as file_size).
- `technology.development.version` — CLEAN→GATED: `N.N.N` collides with dotted dates and 3-part calver (and calver is itself KEPT for that ambiguity); needs a first-part-not-a-year gate.
- `technology.cryptographic.hash` — CLEAN→GATED: the 32-hex branch collides with hyphenless-uuid/md5/tsid under identical validators, so tightest-validator-wins can't break the tie.
- `identity.medical.dea_number` — CLEAN→GATED: weighted check digit unwired, so format-only 2-letter+7-digit collides with license/registration IDs.
- `datetime.date.ordinal` — CLEAN→GATED: `\d{4}-\d{3}` collides with NNNN-NNN lot/product codes and the day group is unbounded.
- `identity.government.ein` — →KEEP: NN-NNNNNNN has no check digit and collides with ssn/tin/account numbers; header-only recovery fragile across shared tax-id headers (author-flagged RECALL_LOSS_RISK).
- `identity.government.ssn` — →KEEP: bare-9-digit branch collides with 9-digit integers, no checksum, PII-sensitive (author-flagged).
- `identity.person.gender_code` — →KEEP: single-char M/F/X/0/1/2/9 collides heavily with boolean/rating/categorical/state_code/initials.
- `identity.medical.cpt` — →KEEP: `\d{5}` branch matches any 5-digit number and collides with postal_code.
- `representation.boolean.initials` — →KEEP: single-char T/F/Y/N collides with grades/gender_code/categorical.
- `geography.location.country_code` — →KEEP: `country_code_corroboration` only fires when the label is already region/city/country, so it cannot recover a lone ISO-2 column the reshaped model files as categorical, and 2-letter members overlap state_code.
- The 8 bare-number epoch/compact-date leaves held at RECALL_LOSS_RISK inside CEDE_GATED (recommended to KEEP in §3b) — anchor removed by the reshape.

## 6. Recommendation

**Take the 134-leaf conservative safe-to-cede set into ac-1, dropping the model from 244 to 110 classes, and ship it behind one new generic recovery rule.** The asymmetry in the validation layer today is the whole risk: `evaluate_validation_veto` and the veto fallback only ever DEMOTE, so every leaf that actually re-asserts is bespoke. Build the generic recogniser — *"assert the leaf whose CLOSED validator passes ≥90% when no tighter or foreign label already fits, tightest-validator-wins to break sibling ties"* — which covers all 100 GENERIC leaves (and subsumes a checksum-PROMOTE for the collision-free checksum leaves iban, vin, figi, bitcoin_address, iso6346, dea_number). Hold the 8 RECALL_LOSS_RISK epoch/compact leaves as KEEP, and hold the 21 NEEDS_NEW_RULE checksum-identifier leaves until their per-type checksum-promote rules land.

**ac-1 NEEDS_NEW_RULE backlog (21):** aba_routing, bsb, swift_bic, credit_card_number, cusip, isin, lei, sedol, orcid, ean, issn, upc, abn, hcpcs, loinc, ndc, npi, blood_type, file_size, dna_sequence, rna_sequence.

**Next action:** build the generic closed-validator recovery rule (tightest-validator-wins) and gate-test it against gold before retraining the model on the 110-class label space.
