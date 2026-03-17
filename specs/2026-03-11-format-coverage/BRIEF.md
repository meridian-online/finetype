# Format Coverage Research Brief

## What is FineType?

[FineType](https://github.com/noon-org/finetype) is an open-source type inference engine that detects and classifies data types in tabular datasets. Given a column of text values, it returns a precise type label (e.g. `datetime.date.iso`, `finance.currency.amount_us`) along with a DuckDB-compatible format string and SQL transform. It currently classifies 163 types across 7 domains using a CharCNN model and a taxonomy defined in YAML. Written in Rust with zero Python dependencies.

## Question

What date, time, timestamp, and currency formats appear commonly in real-world datasets — and what should FineType call each one?

## Why This Matters

FineType's value to analysts depends on **actionable format strings**: when we say a column is `datetime.date.eu_dot`, the analyst gets `%d.%m.%Y` and can immediately parse it. Every format we miss is a column that gets classified as a generic type with no transform — the analyst has to figure it out themselves.

We currently cover 45 datetime types and 4 currency types. CLDR data shows 700+ locales worth of patterns. Real-world CSVs on Kaggle, data.gov, and GitHub show formats we haven't named yet. This research identifies the gaps.

## Scope

### In Scope

1. **Date formats** — all orderings (DMY, MDY, YMD), separators (slash, dash, dot, space), padding (01 vs 1), 2-digit vs 4-digit years
2. **Time formats** — 12h/24h, with/without seconds, with/without fractional seconds, with/without AM/PM
3. **Timestamp formats** — date+time combinations, timezone representations, offset formats
4. **Currency/monetary formats** — symbol position, separator conventions, accounting notation (parentheses for negative), locale-specific grouping

### Out of Scope

- Phone, postal, address formats (covered by Locale Foundation milestone)
- Identifier formats (UUIDs, codes, etc.)
- New domains not yet in taxonomy

## Current Coverage

### Datetime (45 types)

**Dates (19):**
| Type | Format | Example |
|------|--------|---------|
| `iso` | `%Y-%m-%d` | 2024-01-15 |
| `us_slash` | `%m/%d/%Y` | 01/15/2024 |
| `eu_slash` | `%d/%m/%Y` | 15/01/2024 |
| `eu_dot` | `%d.%m.%Y` | 15.01.2024 |
| `compact_ymd` | `%Y%m%d` | 20240115 |
| `compact_mdy` | `%m%d%Y` | 01152024 |
| `compact_dmy` | `%d%m%Y` | 15012024 |
| `short_ymd` | `%y-%m-%d` | 24-01-15 |
| `short_mdy` | `%m-%d-%y` | 01-15-24 |
| `short_dmy` | `%d-%m-%y` | 15-01-24 |
| `long_full_month` | `%B %d, %Y` | January 15, 2024 |
| `abbreviated_month` | `%b %d, %Y` | Jan 15, 2024 |
| `weekday_full_month` | `%A, %B %d, %Y` | Monday, January 15, 2024 |
| `weekday_abbreviated_month` | `%A, %b %d, %Y` | Monday, Jan 15, 2024 |
| `iso_week` | `%G-W%V` | 2024-W03 |
| `julian` | (no format_string) | 2460324 |
| `ordinal` | `%Y-%j` | 2024-015 |
| `day_of_month` | component | 15 |
| `year` | component | 2024 |

**Times (5):**
| Type | Format | Example |
|------|--------|---------|
| `hm_24h` | `%H:%M` | 14:30 |
| `hms_24h` | `%H:%M:%S` | 14:30:00 |
| `hm_12h` | `%I:%M %p` | 02:30 PM |
| `hms_12h` | `%I:%M:%S %p` | 02:30:00 PM |
| `iso` (time) | `%H:%M:%S.%f` | 14:30:00.123456 |

**Timestamps (13):**
| Type | Format | Example |
|------|--------|---------|
| `iso_8601` | `%Y-%m-%dT%H:%M:%SZ` | 2024-01-15T14:30:00Z |
| `iso_8601_microseconds` | `%Y-%m-%dT%H:%M:%S.%fZ` | 2024-01-15T14:30:00.123456Z |
| `iso_8601_offset` | `%Y-%m-%dT%H:%M:%S%z` | 2024-01-15T14:30:00+05:30 |
| `iso_8601_compact` | `%Y%m%dT%H%M%S` | 20240115T143000 |
| `iso_microseconds` | `%Y-%m-%dT%H:%M:%S.%f` | 2024-01-15T14:30:00.123456 |
| `sql_standard` | `%Y-%m-%d %H:%M:%S` | 2024-01-15 14:30:00 |
| `american` | `%m/%d/%Y %I:%M %p` | 01/15/2024 02:30 PM |
| `american_24h` | `%m/%d/%Y %H:%M:%S` | 01/15/2024 14:30:00 |
| `european` | `%d/%m/%Y %H:%M` | 15/01/2024 14:30 |
| `rfc_2822` | `%a, %d %b %Y %H:%M:%S %z` | Mon, 15 Jan 2024 14:30:00 +0000 |
| `rfc_2822_ordinal` | (no format_string) | — |
| `rfc_3339` | `%Y-%m-%d %H:%M:%S%:z` | 2024-01-15 14:30:00+00:00 |
| + 3 epoch types | unix_seconds, unix_milliseconds, unix_microseconds | |

**Other (5):** duration, utc offset, IANA timezone, periodicity, month_name, day_of_week

### Finance Currency (4 types)

| Type | Description |
|------|-------------|
| `amount_us` | `$1,234.56` — comma thousands, dot decimal |
| `amount_eu` | `€1.234,56` — dot thousands, comma decimal |
| `currency_code` | `USD`, `EUR` — ISO 4217 |
| `currency_symbol` | `$`, `€`, `£` |

## Known Gaps

### Date Formats Missing

These appear in real data but have no FineType type:

1. **DMY with space separator** — `15 01 2024`, `15 Jan 2024`
2. **YMD with dot separator** — `2024.01.15` (common in East Asian contexts)
3. **Abbreviated month without comma** — `Jan 15 2024`, `15 Jan 2024`
4. **Month-year only** — `January 2024`, `Jan 2024`, `01/2024`
5. **Quarter notation** — `Q1 2024`, `2024Q1`, `2024-Q1`
6. **Fiscal year** — `FY2024`, `FY24`
7. **Relative dates** — `yesterday`, `3 days ago` (probably out of scope)
8. **Japanese era dates** — `令和6年1月15日` (Reiwa 6, January 15)
9. **Chinese date format** — `2024年1月15日`
10. **Korean date format** — `2024년 1월 15일`

### Timestamp Formats Missing

1. **SQL with microseconds** — `2024-01-15 14:30:00.123456`
2. **European with seconds** — `15/01/2024 14:30:00`
3. **Dot-separated European timestamp** — `15.01.2024 14:30:00`
4. **American with seconds** — `01/15/2024 02:30:00 PM`
5. **Syslog/Common Log Format** — `15/Jan/2024:14:30:00 +0000`
6. **Apache/NCSA combined** — `[15/Jan/2024:14:30:00 +0000]`
7. **ISO 8601 with milliseconds** (3-digit) — `2024-01-15T14:30:00.123Z`
8. **ISO 8601 with space separator** — `2024-01-15 14:30:00Z`

### Currency Formats Missing

1. **Accounting notation** — `(1,234.56)` for negative (no symbol)
2. **Symbol-suffix** — `1,234.56 USD`, `1.234,56 EUR`
3. **Indian numbering** — `₹12,34,567.89` (lakh/crore grouping)
4. **Swiss/Liechtenstein** — `CHF 1'234.56` (apostrophe thousands separator)
5. **Brazilian Real** — `R$ 1.234,56` (multi-char symbol + EU separators)
6. **Japanese Yen** — `¥1,234` (no decimals)
7. **Cryptocurrency amounts** — `0.00123456 BTC`, `1.234567890123456789 ETH` (high precision)
8. **Basis points** — `125 bps`, `25bp`
9. **Percentage with currency context** — `+2.5%` (return/yield)

## Research Questions

For each candidate format, the research agents should determine:

### 1. Prevalence

- How often does this format appear in publicly available datasets (Kaggle, data.gov, GitHub CSVs)?
- Is it specific to one industry/region or broadly used?
- What data sources or applications produce this format?

### 2. Naming Convention

FineType names follow `domain.category.type` with descriptive, lowercase, underscore-separated type names. Names should:

- **Describe the format, not the locale** — `eu_dot` not `german_date`
- **Be unambiguous** — a developer seeing the name should know the format
- **Follow existing patterns** — `compact_*` for no-separator, `short_*` for 2-digit year, `iso_*` for standards
- **Prefer the standard name** when one exists — `rfc_2822`, `iso_8601`

For new types, propose 2-3 candidate names with rationale.

### 3. Distinguishability

- Can this format be reliably distinguished from existing types via character patterns?
- What is the ambiguity surface? (e.g., `01/02/2024` is MDY or DMY — we already handle this)
- Would adding this type increase false positives on existing types?

### 4. Actionability

- What DuckDB `strptime` format string parses this format?
- What `broad_type` should it map to?
- If not strptime-parseable, what SQL transform expression would convert it?

## Research Tasks for Agents

### Task A: Date Format Census

**Sources to check:**
- Unicode CLDR `dateFormatLength` patterns (short/medium/long/full) across top 50 locales
- Kaggle dataset metadata — search for date columns, extract unique formats
- data.gov bulk CSV headers — identify date column patterns
- GitHub CSV corpus (GiTables) — we have 1M column eval, check misclassified date columns
- Python `dateutil.parser` source — what formats does it recognize?
- Pandas `to_datetime` documentation — what `format` strings are commonly used?
- JavaScript `Date.parse()` — what formats does the spec define?
- PostgreSQL `to_date`/`to_timestamp` — what format patterns exist?
- Excel date format codes — what do spreadsheet exports produce?

**Deliverable:** Table of formats with columns: `pattern`, `example`, `proposed_name`, `sources_seen`, `estimated_prevalence`, `ambiguity_notes`

### Task B: Timestamp Format Census

**Sources to check:**
- Log format standards (syslog RFC 5424, Apache, nginx, CloudWatch, Datadog)
- API response formats (REST conventions, GraphQL datetime scalars)
- Database export formats (pg_dump, mysqldump, SQLite, MongoDB BSON)
- Cloud provider timestamp conventions (AWS, GCP, Azure event formats)
- Message queue/event formats (Kafka, RabbitMQ timestamp headers)
- Observability tools (OpenTelemetry, Prometheus, Grafana)

**Deliverable:** Same table format as Task A

### Task C: Currency Format Census

**Sources to check:**
- Unicode CLDR `currencyFormatLength` patterns across top 50 locales
- CLDR `numbers.symbols` (decimal, group, currency separators per locale)
- Financial data APIs (Bloomberg, Reuters, Yahoo Finance) — what formats do they emit?
- Accounting software exports (QuickBooks, Xero, SAP)
- E-commerce platforms (Shopify, Stripe, Square) — receipt/invoice formats
- Central bank data feeds (Federal Reserve, ECB, BOJ)
- International numbering systems (Indian lakh/crore, East Asian wan/oku)

**Deliverable:** Table of formats with columns: `pattern`, `example`, `proposed_name`, `separator_convention`, `symbol_position`, `negative_notation`, `sources_seen`

### Task D: Name Validation

Cross-reference all proposed names against:
- Existing FineType taxonomy (no collisions)
- Common developer terminology (names should be guessable)
- Other type inference tools (Visions, dataprep, pandas-profiling) — how do they name similar types?

## Output Format

Each research agent should produce a markdown file in this directory:

```
specs/format-coverage/
  BRIEF.md              (this file)
  dates.md              (Task A output)
  timestamps.md         (Task B output)
  currencies.md         (Task C output)
  name-validation.md    (Task D output)
  SUMMARY.md            (consolidated findings + recommendations)
```

## Success Criteria

- [ ] Identified 10+ date/time formats not currently in the taxonomy
- [ ] Identified 5+ currency formats not currently in the taxonomy
- [ ] Each proposed format has: name, pattern, example, DuckDB format string, prevalence estimate
- [ ] No proposed name collides with existing taxonomy
- [ ] Names follow FineType naming conventions
- [ ] Ambiguity analysis for formats that overlap with existing types
- [ ] Clear recommendation on which formats to add (prioritized by prevalence)

## Time Budget

~4 hours total across research agents. Each task (A-D) should take 30-60 minutes of web research + synthesis.

## Context for Research Agents

FineType classifies individual text values and columns into a taxonomy of 163 types. Each type maps to a DuckDB SQL type and includes a transform expression. The taxonomy lives in YAML files under `labels/`. Type names use dotted hierarchy: `domain.category.type` (e.g., `datetime.date.iso`, `finance.currency.amount_us`).

When reading CLDR data, note that FineType already has the raw CLDR extracts in `data/cldr/` — the research should focus on formats that appear in *actual datasets* and aren't covered, not on exhaustively cataloging every CLDR pattern.
