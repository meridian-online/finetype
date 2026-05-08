# Taxonomy Revision Recommendation — v0.5.1

**Task:** NNFT-176
**Status:** Recommendation
**Date:** 2026-03-02

## Executive Summary

This document recommends specific taxonomy changes for FineType v0.5.1, informed by external research (two independent agents surveying Kaggle, government data, enterprise SaaS schemas, and analyst pain points) and validated against the actual v0.5.0 taxonomy (163 types).

**Correction note:** The research brief's appendix was out of date (pre-NNFT-162), causing both research agents to flag phantom duplicates and missing types that already exist. This document corrects those errors and presents only validated recommendations.

**Net result:** Remove 2 types, add 5-7 types, restructure 1 domain, create 1 new category + 1 new commerce category. Total taxonomy moves from 163 to ~166-168 types.

---

## Guiding Principles

These principles were validated by both research sources and Hugh's review:

1. **Each type is a transformation contract** — if FineType says it's a type, the corresponding DuckDB cast WILL succeed. Types that produce the same DuckDB expression should merge; types that produce different expressions deserve to exist.

2. **Format strings are sacred** — the 40 datetime types with distinct `format_string` values each solve the #4 analyst frustration (date format ambiguity). The datetime domain should grow, not consolidate.

3. **Categoricals are a superpower** — FineType detecting `categorical` + DuckDB's native `ENUM` type is a genuine competitive advantage no other tool offers.

4. **Precision over permissiveness** — a type that confirms 90% of random input is not a valid type. Types must meaningfully distinguish "is this" from "is not this."

---

## Research Corrections

The external research was conducted against an outdated type list. The following findings from both research agents were **invalid**:

| Research finding | Correction |
|---|---|
| "UUID is MISSING" | `technology.cryptographic.uuid` exists (UUID broad_type, priority 5) |
| "ISO country code MISSING" | `geography.location.country_code` exists (priority 5) |
| "Language/locale tag MISSING" | `technology.code.locale_code` exists (priority 4) |
| "Duplicate email (identity + technology)" | Only `identity.person.email` exists |
| "Duplicate boolean (representation + technology)" | Only `representation.boolean.*` exists |
| "Duplicate entity_name (identity + representation)" | Only `representation.text.entity_name` exists |
| "EAN/ISBN/ISSN in identity.payment" | They are in `technology.code` |
| "chemical_formula, scientific_notation in rep.scientific" | `rep.scientific` contains dna_sequence, measurement_unit, metric_prefix, protein_sequence, rna_sequence |

**Valid findings from research** (confirmed against actual taxonomy):
- Currency amount with symbol: genuinely missing
- Locale-aware number (European decimal format): genuinely missing
- HTML content type: genuinely missing
- `identity.payment` mixes securities + crypto + payment: confirmed, 14 types
- `datetime.component.century`: confirmed low-value (Roman numerals only)
- `identity.payment.cvv`: confirmed high false-positive rate

---

## Recommendations

### 1. Removals (2 types)

#### Remove `identity.payment.cvv`
- **Rationale:** 3-4 digit integers. Extremely high false-positive rate — any column of small numbers can match. Low analyst value (CVVs should never appear in analytical datasets). Security concern.
- **Impact:** Model retraining data updated, one fewer label. No eval baseline affected (not in profile eval).

#### Remove `datetime.component.century`
- **Rationale:** Detects Roman numerals (XIX, XX, XXI). No `format_string`, no DuckDB transformation contract. Vanishingly rare as a standalone column. Classification as `representation.discrete.categorical` would be more accurate.
- **Impact:** Minimal. One fewer label.

### 2. New Types (5-7 types)

#### Add currency amount types

The #1 gap identified by both research sources. Currency amounts with symbols (`$1,234.56`, `€1.234,56`) appear in virtually every business dataset and are the #6 most common analyst frustration.

Following the datetime principle — each format produces a distinct DuckDB transform:

| Proposed type | Pattern | DuckDB transform |
|---|---|---|
| `finance.currency.amount_us` | `$1,234.56`, `-$1,234.56` | `REPLACE(REPLACE(REPLACE(col, '$',''), ',',''), ' ','')::DECIMAL(18,2)` |
| `finance.currency.amount_eu` | `€1.234,56`, `1.234,56 €` | `REPLACE(REPLACE(col, '.',''), ',','.')::DECIMAL(18,2)` (after symbol strip) |
| `finance.currency.amount_accounting` | `$(1,234.56)` | Parentheses → negative, then US transform |

These sit naturally in the new `finance` domain (see Recommendation 4).

**Open design question:** Locale detection for currency amounts parallels the datetime locale model. The currency symbol identifies the locale; the separator pattern determines the transform. Detailed design should be a separate task.

#### Add `container.object.html`

- **Rationale:** HTML is NOT well-formed XML. HTML5 allows unclosed tags, unquoted attributes, optional closing. `<p>Hello` is valid HTML, invalid XML. Common in CMS exports (Shopify, HubSpot), email templates, web scraping data.
- **DuckDB transform:** `regexp_replace(col, '<[^>]+>', '', 'g')` for basic tag stripping. DuckDB's [webbed extension](https://duckdb.org/community_extensions/extensions/webbed) enables richer HTML/DOM operations.
- **Detection:** Presence of HTML tags (`<p>`, `<div>`, `<a href=`, `<br>`, etc.) distinguishes from XML (which requires well-formedness).

#### Add `finance.banking.iban`

- **Rationale:** International Bank Account Number (ISO 13616). Up to 34 alphanumeric characters with country prefix and check digits. Common in international financial datasets. Was listed in the research brief as an existing type but is genuinely missing from v0.5.0.
- **DuckDB transform:** `CAST(col AS VARCHAR)` — validation-focused type. The check digit algorithm (mod-97) provides strong detection signal.
- **Detection:** 2-letter country code + 2 check digits + up to 30 alphanumeric characters. Strong structural pattern.

#### Add locale-aware number (European decimal format)

- **Rationale:** European format `1.234,56` uses period for thousands, comma for decimal — the exact inverse of US format. Silent corruption risk: `1.234` is either 1234 (European) or 1.234 (US). #7 analyst frustration.
- **Proposed type:** `representation.numeric.decimal_number_eu` or similar
- **DuckDB transform:** `REPLACE(REPLACE(col, '.', ''), ',', '.')::DOUBLE`
- **Design consideration:** Needs column-level detection (single values are ambiguous). The Sense model's locale awareness may help here.

### 3. New Category: `representation.identifier`

Per Hugh's decision, create a new category grouping types that indicate "this column is a key/identifier":

| Type | Current location | New location |
|---|---|---|
| `uuid` | `technology.cryptographic.uuid` | `representation.identifier.uuid` |
| `alphanumeric_id` | `representation.code.alphanumeric_id` | `representation.identifier.alphanumeric_id` |
| `increment` | `representation.numeric.increment` | `representation.identifier.increment` |

**Rationale:** UUID appears in database design far beyond technology contexts. Increment (monotonic increasing sequences) is a fundamental database concept. Alphanumeric IDs are identifiers by definition. Grouping them tells analysts "this column is a key."

**Impact:** Three type label changes. Model retraining needed. Eval baselines updated. The `representation.code` category becomes empty and is removed.

### 4. New Domain: `finance`

Restructure `identity.payment` (14 types) into a new top-level `finance` domain. The current grouping mixes payment instruments, securities identifiers, cryptocurrency addresses, and currency metadata under a single "payment" category.

**Proposed structure:**

```
finance/
  banking/
    iban               (NEW — ISO 13616, genuinely missing from taxonomy)
    swift_bic          (moved from identity.payment)
  payment/
    credit_card_number
    credit_card_expiration_date
    credit_card_network
    paypal_email
  securities/
    cusip
    isin
    sedol
    lei
  crypto/
    bitcoin_address
    ethereum_address
  currency/
    currency_code
    currency_symbol
    amount_us          (NEW)
    amount_eu          (NEW)
    amount_accounting  (NEW — if implemented)
```

Additionally, move product/publication identifiers to a new commerce category in `identity`:

```
identity/
  commerce/
    ean                (moved from technology.code)
    isbn               (moved from technology.code)
    issn               (moved from technology.code)
  medical/
    dea_number, ndc, npi  (unchanged)
  person/
    14 types              (unchanged)
```

**Types removed from the restructure:**
- `cvv` — removed entirely (Recommendation 1)

**Rationale:**
- Securities identifiers (CUSIP, ISIN, SEDOL, LEI) are governed by ISO standards for trading/clearing, not payments. Every financial data analyst would look for these separately.
- Banking identifiers (SWIFT/BIC, IBAN) are international banking standards — distinct from payment instruments.
- Cryptocurrency addresses are a distinct verification pattern.
- Currency types (codes, symbols, amounts) form a coherent family.
- Product/publication identifiers (EAN, ISBN, ISSN) are commerce codes, not technology codes. `identity.commerce.product` groups them where analysts would look.
- Both research sources independently identified the payment restructure as "urgently needed."

**Impact:** This is the highest-impact structural change. Affects:
- Taxonomy YAML files (new domain file `labels/definitions_finance.yaml`, updated `labels/definitions_identity.yaml`, updated `labels/definitions_technology.yaml`)
- Sense model category labels and `LabelCategoryMap`
- Training data labels
- Eval baselines
- CLAUDE.md documentation

The `identity` domain changes from 31 to 20 types (person: 14, medical: 3, commerce: 3). The `technology.code` category shrinks from 7 to 4 types (doi, imei, locale_code, pin).

### 5. Keep and Celebrate

Types that both research sources or Hugh flagged for removal/demotion but should **stay**:

| Type | Why keep |
|---|---|
| `datetime.component.periodicity` | Detects "Daily", "Monthly", "Quarterly" — casts to ENUM. Hugh says keep. |
| `datetime.component.day_of_week` | Locale-specific detection. Casts to ENUM. Has analytical utility. |
| `datetime.component.month_name` | Locale-specific detection. Analytical utility. |
| `datetime.component.year` | Time-series filtering. `SMALLINT` cast. |
| `datetime.component.day_of_month` | `TINYINT` cast. Analytical utility. |
| `representation.scientific.*` | DNA/RNA/protein sequences serve bioinformatics. Measurement units have broad utility. |
| `representation.numeric.scientific_notation` | While DuckDB handles E-notation in CAST, detecting it lets us preserve precision and warn about overflow. |
| All 40 format-string datetime types | Each is a distinct transformation contract. |

### 6. Naming Review

| Current name | Issue | Recommendation |
|---|---|---|
| `representation.numeric.si_number` | "SI number" is not analyst-intuitive. | Rename to `formatted_size` or `si_prefix_number`. Defer to implementation task. |
| `representation.numeric.increment` | "Increment" is ambiguous (delta? sequence? counter?) | Moving to `representation.identifier.increment` helps. Consider renaming to `sequence` or `auto_increment`. |

---

## What We're NOT Doing (and Why)

These were suggested by research but rejected:

| Suggestion | Why not |
|---|---|
| Compress datetime from 46 to 18 types | The 40 format-string types are distinct contracts. Only the 6 components were reviewed. |
| Add "score/rating" type (4.5/5) | Rare as a column type. Usually just floats. Low detection signal. |
| Add "rate per N" type | Semantic metadata, not a detectable format. Looks identical to a plain float. |
| Add "masked/redacted value" | `[REDACTED]` has no reliable pattern. Too many formats. |
| Add "Reference ID / Lookup" type | Relational/structural concept, not detectable from column values. |
| Add "Status/Priority Enum" | Already covered by `representation.discrete.categorical`. |
| Add "FIPS/GNIS codes" | Too US-specific for a general taxonomy. |
| Add "Fraction" type (1/4, 3/8) | Medium frequency, ambiguous with dates. Lower priority. |
| Add "File path" type | Partially covered by existing file types. Lower priority. |
| Create "measurement" domain | Conceptually appealing but large restructuring cost for limited benefit. Currency types go in finance domain instead. |

---

## Version and Release Strategy

**Target:** v0.5.1 (patch release — taxonomy refinement, not a major version bump)

**Implementation order** (each step validated before proceeding):

1. **Removals** — CVV, century. Simplest change, reduces taxonomy. Retrain model.
2. **New category** — `representation.identifier` (move UUID, alphanumeric_id, increment). Label changes only.
3. **Restructure** — New `finance` domain (from identity.payment) + `identity.commerce` (move EAN/ISBN/ISSN from technology.code). New YAML files, label migration.
4. **New types** — IBAN, currency amounts, HTML, locale-aware number. New YAML definitions, generators, training data.
5. **Naming fixes** — si_number rename, increment rename if agreed.

Each step requires: YAML change → generator check → model retrain → eval baseline update → smoke tests.

---

## Appendix: Current Taxonomy (v0.5.0, 163 types)

| Domain | Category | Count | Types |
|---|---|---|---|
| container | array | 4 | comma_separated, pipe_separated, semicolon_separated, whitespace_separated |
| container | key_value | 2 | form_data, query_string |
| container | object | 5 | csv, json, json_array, xml, yaml |
| datetime | component | 6 | century, day_of_month, day_of_week, month_name, periodicity, year |
| datetime | date | 17 | abbreviated_month, compact_dmy, compact_mdy, compact_ymd, eu_dot, eu_slash, iso, iso_week, julian, long_full_month, ordinal, short_dmy, short_mdy, short_ymd, us_slash, weekday_abbreviated_month, weekday_full_month |
| datetime | duration | 1 | iso_8601 |
| datetime | epoch | 3 | unix_microseconds, unix_milliseconds, unix_seconds |
| datetime | offset | 2 | iana, utc |
| datetime | time | 5 | hm_12h, hm_24h, hms_12h, hms_24h, iso |
| datetime | timestamp | 12 | american, american_24h, european, iso_8601, iso_8601_compact, iso_8601_microseconds, iso_8601_offset, iso_microseconds, rfc_2822, rfc_2822_ordinal, rfc_3339, sql_standard |
| geography | address | 5 | full_address, postal_code, street_name, street_number, street_suffix |
| geography | contact | 1 | calling_code |
| geography | coordinate | 3 | coordinates, latitude, longitude |
| geography | location | 5 | city, continent, country, country_code, region |
| geography | transportation | 2 | iata_code, icao_code |
| identity | medical | 3 | dea_number, ndc, npi |
| identity | payment | 14 | bitcoin_address, credit_card_expiration_date, credit_card_network, credit_card_number, currency_code, currency_symbol, cusip, cvv, ethereum_address, isin, lei, paypal_email, sedol, swift_bic |
| identity | person | 14 | age, blood_type, email, first_name, full_name, gender, gender_code, gender_symbol, height, last_name, password, phone_number, username, weight |
| representation | boolean | 3 | binary, initials, terms |
| representation | code | 1 | alphanumeric_id |
| representation | discrete | 2 | categorical, ordinal |
| representation | file | 4 | excel_format, extension, file_size, mime_type |
| representation | numeric | 6 | decimal_number, increment, integer_number, percentage, scientific_notation, si_number |
| representation | scientific | 5 | dna_sequence, measurement_unit, metric_prefix, protein_sequence, rna_sequence |
| representation | text | 8 | color_hex, color_rgb, emoji, entity_name, paragraph, plain_text, sentence, word |
| technology | code | 7 | doi, ean, imei, isbn, issn, locale_code, pin |
| technology | cryptographic | 4 | hash, token_hex, token_urlsafe, uuid |
| technology | development | 6 | calver, os, programming_language, software_license, stage, version |
| technology | hardware | 2 | ram_size, screen_size |
| technology | internet | 11 | hostname, http_method, http_status_code, ip_v4, ip_v4_with_port, ip_v6, mac_address, port, top_level_domain, url, user_agent |
