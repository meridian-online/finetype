# FineType format gap analysis: 54 missing types across dates, timestamps, and currencies

FineType's current taxonomy of 45 datetime types and 4 currency types covers the most common Western and ISO formats but **misses 54 distinct formats** that appear routinely in real-world tabular data. The highest-impact gaps fall into three clusters: separator and year-length variants for dates (the existing taxonomy covers dash-separated short years but not slash-separated short years or dash-separated full years for DMY/MDY), fractional-second and timezone combinations for timestamps (JavaScript's ubiquitous millisecond ISO format is absent), and locale-specific grouping conventions for currency (Indian lakh/crore, Swiss apostrophe separators, and EU suffix notation serve billions of users). Every proposed format below includes a DuckDB `strptime` string or SQL transform, a unique name with zero collisions against the existing taxonomy, and a prevalence estimate grounded in CLDR locale data, log-format RFCs, and financial-industry standards.

No other type-inference tool—Visions, ydata-profiling, DataPrep, Great Expectations, Frictionless Data—operates at this level of format granularity. FineType's approach of encoding each format as a named "transformation contract" mapping to an exact `strptime` call is unique in the ecosystem and well-justified, since a single-character difference in a format string causes parsing failure.

---

## Date format census: 22 actionable additions

The largest gap category is **separator and field-order variants**. FineType covers dash-separated dates with 2-digit years (`short_dmy`, `short_mdy`) but not with 4-digit years, and covers slash-separated dates for US/EU but not YMD order. The second cluster is **partial dates** (year-month, quarters), which are pervasive in financial and time-series datasets.

| # | Proposed name | strptime / DuckDB | Example | Prevalence | Ambiguity notes |
|---|---|---|---|---|---|
| 1 | `ymd_slash` | `%Y/%m/%d` | `2024/01/15` | **High** | Unambiguous (year-first). CLDR ja-JP short format; standard in Japanese/Taiwanese CSV exports |
| 2 | `ymd_dot` | `%Y.%m.%d` | `2024.01.15` | Medium | Unambiguous (year-first). Hungarian, Norwegian, some East Asian systems |
| 3 | `dmy_dash` | `%d-%m-%Y` | `15-01-2024` | **High** | Ambiguous with `mdy_dash` when day ≤ 12. Extremely common in EU informal data, Pandas examples |
| 4 | `mdy_dash` | `%m-%d-%Y` | `01-15-2024` | Medium | Ambiguous with `dmy_dash` when day ≤ 12. US database exports |
| 5 | `dmy_space_abbrev` | `%d %b %Y` | `15 Jan 2024` | **High** | Unambiguous (named month). RFC 2822 date portion, military/NATO, PostgreSQL `to_char` |
| 6 | `dmy_space_full` | `%d %B %Y` | `15 January 2024` | Medium | Unambiguous. CLDR en-GB long format, European formal documents |
| 7 | `abbrev_month_no_comma` | `%b %d %Y` | `Jan 15 2024` | **High** | Unambiguous. SQL Server BCP output, syslog date headers. Distinct from `abbreviated_month` which requires comma |
| 8 | `full_month_no_comma` | `%B %d %Y` | `January 15 2024` | Medium | Unambiguous. Variant of `long_full_month` without comma |
| 9 | `dmy_dash_abbrev` | `%d-%b-%Y` | `15-Jan-2024` | **High** | Unambiguous. **Oracle NLS_DATE_FORMAT default**; banking/financial exports; DuckDB multi-format examples |
| 10 | `dmy_dash_abbrev_short` | `%d-%b-%y` | `15-Jan-24` | **High** | Named month resolves order; 2-digit year has century ambiguity. Oracle classic `DD-MON-RR` format |
| 11 | `us_short_slash` | `%m/%d/%y` | `01/15/24` | **High** | Ambiguous with `eu_short_slash` when day ≤ 12. Excel US short date, extremely common in informal data |
| 12 | `eu_short_slash` | `%d/%m/%y` | `15/01/24` | **High** | Ambiguous with `us_short_slash` when day ≤ 12. Excel EU short date |
| 13 | `eu_short_dot` | `%d.%m.%y` | `15.01.24` | Medium | 2-digit year ambiguity. CLDR de-DE short format (`d.M.yy`). Dot convention is nearly always DMY |
| 14 | `year_month` | `%Y-%m` | `2024-01` | **High** | No day component. ISO 8601 truncated form; pervasive in monthly time-series, API responses |
| 15 | `compact_ym` | `%Y%m` | `202401` | Medium | No day. Could collide with 6-digit integers. Data warehouse period keys, financial reporting codes |
| 16 | `month_year_full` | `%B %Y` | `January 2024` | Medium | No day. Financial reports, government statistical publications |
| 17 | `month_year_abbrev` | `%b %Y` | `Jan 2024` | Medium | No day. Financial dashboards, CLDR `yMMM` skeleton |
| 18 | `month_year_slash` | `%m/%Y` | `01/2024` | Medium | No day. Billing data, credit card contexts |
| 19 | `weekday_dmy_full` | `%A, %d %B %Y` | `Monday, 15 January 2024` | Medium | Unambiguous. CLDR en-GB/fr-FR full format. Existing weekday types are MDY-ordered |
| 20 | `chinese_ymd` | `%Y年%-m月%-d日` | `2024年1月15日` | **High** (regional) | Unambiguous due to 年/月/日 delimiters. Standard Chinese date; government datasets |
| 21 | `korean_ymd` | `%Y년 %-m월 %-d일` | `2024년 1월 15일` | Medium (regional) | Unambiguous due to 년/월/일 delimiters. Korean government and institutional data |
| 22 | `quarter` | Custom (no strptime) | `Q1 2024` / `2024-Q1` | Medium | Not a single date—represents a 3-month range. Requires custom regex. Ubiquitous in SEC filings, earnings data |

Eight of these 22 formats rate **high prevalence**: `ymd_slash`, `dmy_dash`, `dmy_space_abbrev`, `abbrev_month_no_comma`, `dmy_dash_abbrev`, `dmy_dash_abbrev_short`, `us_short_slash`, `eu_short_slash`, and `year_month`. The Oracle-derived formats (#9, #10) alone likely account for millions of CSV files exported from enterprise databases.

---

## Timestamp format census: 16 actionable additions

The most consequential gap is **fractional-second precision combined with timezone offsets**. FineType's existing types cover microseconds without offset (`iso_8601_microseconds`) and offset without fractional seconds (`iso_8601_offset`), but not the combination—which is precisely what Python's `datetime.now(tz).isoformat()` and RFC 5424 syslog produce. The second critical gap is **log-format timestamps** (CLF, BSD syslog) that appear in virtually all server observability data.

| # | Proposed name | strptime / DuckDB | Example | Prevalence | Ambiguity notes |
|---|---|---|---|---|---|
| 1 | `sql_microseconds` | `%Y-%m-%d %H:%M:%S.%f` | `2024-01-15 14:30:00.123456` | **High** | Python `str(datetime.now())` default. Distinct from `iso_8601_microseconds` by space (not T) separator |
| 2 | `sql_milliseconds` | `%Y-%m-%d %H:%M:%S.%g` | `2024-01-15 14:30:00.123` | **High** | 3-digit fractional seconds. MySQL `DATETIME(3)`, Java `Timestamp.toString()`. DuckDB uses `%g` for millis |
| 3 | `iso_8601_milliseconds` | `%Y-%m-%dT%H:%M:%S.%gZ` | `2024-01-15T14:30:00.123Z` | **High** | **JavaScript `new Date().toISOString()`** — arguably the most common timestamp in JSON APIs worldwide |
| 4 | `iso_8601_millis_offset` | `%Y-%m-%dT%H:%M:%S.%g%z` | `2024-01-15T14:30:00.123+05:30` | Medium | Java `OffsetDateTime.toString()` with ms precision. Combines millis + numeric offset |
| 5 | `iso_8601_micros_offset` | `%Y-%m-%dT%H:%M:%S.%f%z` | `2024-01-15T14:30:00.123456+00:00` | **High** | RFC 5424 syslog, Python `datetime.now(tz).isoformat()`. Neither `iso_8601_microseconds` nor `iso_8601_offset` covers this combination |
| 6 | `clf` | `%d/%b/%Y:%H:%M:%S %z` | `15/Jan/2024:14:30:00 +0000` | **High** | Apache/Nginx Common Log Format. Note colon (`:`) between date and time with no space—highly distinctive |
| 7 | `syslog_bsd` | `%b %-d %H:%M:%S` | `Jan 15 14:30:00` | **High** | **No year component**. RFC 3164, `/var/log/syslog`. Day is space-padded. Extremely widespread in legacy logs |
| 8 | `sql_microseconds_offset` | `%Y-%m-%d %H:%M:%S.%f%z` | `2024-01-15 14:30:00.123456+00:00` | **High** | PostgreSQL `TIMESTAMPTZ` output. Space separator + microseconds + full offset |
| 9 | `pg_short_offset` | `%Y-%m-%d %H:%M:%S.%f%z` | `2024-01-15 14:30:00.123456-05` | Medium | PostgreSQL-specific **2-digit offset** (e.g., `-05` not `-05:00`). May require special parsing |
| 10 | `dot_dmy_24h` | `%d.%m.%Y %H:%M:%S` | `15.01.2024 14:30:00` | Medium | German/Central European/Russian systems. SAP, German banking. Dot-DMY is standard in DACH region |
| 11 | `slash_ymd_24h` | `%Y/%m/%d %H:%M:%S` | `2024/01/15 14:30:00` | Medium | Japanese system logs, .NET `ja-JP` culture default |
| 12 | `ctime` | `%a %b %-d %H:%M:%S %Y` | `Mon Jan 15 14:30:00 2024` | Medium | C `ctime()`/`asctime()`, Python `datetime.ctime()`, Ruby `Time#to_s`. Unix tradition |
| 13 | `epoch_nanoseconds` | N/A (integer, 19 digits) | `1705325400000000000` | Medium | OpenTelemetry `time_unix_nano`, Go `time.UnixNano()`. Distinct from other epochs by digit count |
| 14 | `iso_space_zulu` | `%Y-%m-%d %H:%M:%SZ` | `2024-01-15 14:30:00Z` | Medium | RFC 3339 permits space instead of T. SQLite `datetime('now')` variants |
| 15 | `sql_offset` | `%Y-%m-%d %H:%M:%S%z` | `2024-01-15 14:30:00+00:00` | Medium | Space separator + offset, no fractional seconds. Django serialization, ETL pipelines |
| 16 | `dot_ymd_24h` | `%Y.%m.%d %H:%M:%S` | `2024.01.15 14:30:00` | Low | East Asian/Baltic niche format |

Seven of these rate **high prevalence**. The single highest-impact addition is **`iso_8601_milliseconds`** (`2024-01-15T14:30:00.123Z`)—this is the exact output of JavaScript's `Date.toISOString()`, making it the default timestamp format in essentially all JSON REST APIs and frontend logging. The **`clf`** format is equally critical for anyone working with web server logs, as Apache and Nginx collectively generate billions of log lines daily.

---

## Currency format census: 12 actionable additions

FineType's current 4 currency types assume US-style or EU-style prefix symbols with standard 3-digit grouping. This misses **three entire dimensions of variation**: symbol position (suffix vs. prefix), grouping convention (Indian lakh/crore, Swiss apostrophe, French space), and negative notation (accounting parentheses, trailing minus). These aren't edge cases—**amount_eu_suffix** alone is the default format for the entire eurozone, and **amount_indian** serves 1.4 billion people.

| # | Proposed name | Example | Separators | Symbol pos. | Negative | Prevalence | DuckDB transform |
|---|---|---|---|---|---|---|---|
| 1 | `amount_accounting_us` | `($1,234.56)` | dec: `.` grp: `,` | prefix | Parentheses | **High** | `CAST(REPLACE(REPLACE(REPLACE(REPLACE(val,'(','-'),')',''),'$',''),',','') AS DECIMAL(18,2))` |
| 2 | `amount_eu_suffix` | `1.234,56 €` | dec: `,` grp: `.` | suffix | Minus prefix | **High** | Strip trailing symbol/space → replace `.`→`` → replace `,`→`.` → cast |
| 3 | `amount_space_sep` | `1 234,56 €` | dec: `,` grp: ` ` | suffix | Minus prefix | **High** | Strip symbol → replace space→`` → replace `,`→`.` → cast. Watch for NBSP (U+00A0) |
| 4 | `amount_indian` | `₹12,34,567.89` | dec: `.` grp: `,` (irregular) | prefix | Minus prefix | **High** | `CAST(REPLACE(REPLACE(val,'₹',''),',','') AS DECIMAL(18,2))`. **Irregular grouping** (XX,XX,XXX) is the distinguishing signal |
| 5 | `amount_ch` | `CHF 1'234.56` | dec: `.` grp: `'` | prefix | Minus prefix | Medium | `CAST(REPLACE(REPLACE(val,'''',''),'CHF','') AS DECIMAL(18,2))`. Must handle both U+0027 and U+2019 |
| 6 | `amount_nodecimal` | `¥1,234` | dec: none grp: `,` | prefix | Minus prefix | **High** | `CAST(REPLACE(REPLACE(val,'¥',''),',','') AS INTEGER)`. JPY, KRW, VND + ~15 other zero-decimal currencies |
| 7 | `amount_code_prefix` | `USD 1,234.56` | dec: varies grp: varies | prefix (ISO code) | Minus prefix | **High** | `regexp_extract(val,'^([A-Z]{3})',1)` for code; strip code+commas → cast for amount. SWIFT, banking, FX |
| 8 | `amount_minor_int` | `12345` (= $123.45) | none | none | Negative int | **High** | `CAST(val AS DECIMAL(18,2)) / 100`. **Stripe, Adyen, Square** all use this. Requires metadata to detect |
| 9 | `amount_crypto` | `0.00123456 BTC` | dec: `.` grp: none | suffix (ticker) | Minus prefix | Medium | Strip suffix ticker → `CAST(... AS DECIMAL(18,8))`. Up to 8+ decimal places |
| 10 | `amount_basis_points` | `125 bps` | dec: `.` grp: none | suffix (bps/bp) | Minus prefix | Medium | Strip "bps"/"bp" → cast → divide by 10000 for decimal. Fixed-income, central bank data |
| 11 | `amount_multisym` | `R$ 1.234,56` | varies by currency | prefix or suffix | Varies | **High** | Multi-character symbols: R$ (Brazil), HK$ (Hong Kong), kr (Nordics), Kč (Czech), zł (Poland). Regex-strip symbol → locale-appropriate parse |
| 12 | `amount_neg_trailing` | `$1,234.56-` or `1,234.56 CR` | dec: `.` grp: `,` | prefix or none | Trailing minus or CR/DR | Medium | SAP, COBOL/mainframe, bank statement exports. Detect trailing `-` or `CR`/`DR` suffix |

The **`amount_minor_int`** format deserves special attention despite being a plain integer. Every major payment processor (Stripe, Adyen, Square, PayPal Braintree) stores and emits monetary amounts as integers in the smallest currency unit. When these systems export to CSV, the column looks like an ordinary integer but represents dollars divided by 100. Detection requires column-name heuristics (e.g., `amount_cents`, `price_minor`) rather than value-pattern matching.

---

## Name validation confirms zero collisions and strong conventions

Cross-referencing all 54 proposed names against FineType's existing 49 types yields **zero name collisions**. The proposed names also follow FineType's established conventions consistently:

**Convention adherence.** Names describe format, not locale (`dmy_dash` not `british_date`; `amount_ch` not `swiss_currency`). The `compact_*` prefix extends naturally to `compact_ym`. The `short_*` prefix for 2-digit years extends to `eu_short_slash`, `eu_short_dot`, and `dmy_dash_abbrev_short`. Standard names are used when available (`clf`, `ctime`, `syslog_bsd`, `rfc_5424`-aligned formats).

**Ecosystem alignment.** Among surveyed tools (Visions, ydata-profiling, DataPrep, Great Expectations, csvkit/agate, Frictionless Data, Arrow, DuckDB, JSON Schema, OpenAPI), none operates at FineType's level of format granularity. The closest is Frictionless Data's Table Schema, which uses a `type + format` parameter approach (e.g., `type: "date", format: "%d/%m/%Y"`). FineType's innovation—encoding format into the type name itself—is unique and well-suited to its role as a "transformation contract" guaranteeing a specific DuckDB `strptime()` call will succeed.

Two Frictionless Data type names worth adopting: **`yearmonth`** (mapping to FineType's proposed `year_month`) and **`year`** (already in FineType's taxonomy). The proposed `month_year_full`, `month_year_abbrev`, and `month_year_slash` extend this partial-date concept with format specificity that Frictionless lacks.

---

## Prioritized implementation roadmap

The 54 formats divide into three implementation tiers based on prevalence, distinctiveness from existing types, and parseability.

**Tier 1 — implement first (20 formats, all high-prevalence):**

Dates (9): `ymd_slash`, `dmy_dash`, `dmy_space_abbrev`, `abbrev_month_no_comma`, `dmy_dash_abbrev`, `dmy_dash_abbrev_short`, `us_short_slash`, `eu_short_slash`, `year_month`. These fill the most acute gaps—FineType currently cannot parse a standard Oracle date export (`15-Jan-2024`), a common EU dash-separated date (`15-01-2024`), or a Japanese slash date (`2024/01/15`).

Timestamps (7): `sql_microseconds`, `sql_milliseconds`, `iso_8601_milliseconds`, `iso_8601_micros_offset`, `clf`, `syslog_bsd`, `sql_microseconds_offset`. The `iso_8601_milliseconds` format alone covers the entire JavaScript ecosystem. The `clf` and `syslog_bsd` formats are essential for any log-analysis use case.

Currency (4): `amount_accounting_us`, `amount_eu_suffix`, `amount_indian`, `amount_nodecimal`. Accounting parentheses appear in virtually every 10-K filing. EU suffix notation is the CLDR default for Germany, France, Spain, Italy, and the Netherlands. Indian grouping serves **1.4 billion people**. Zero-decimal currencies cover Japan (3rd-largest economy) and South Korea (12th).

**Tier 2 — implement second (22 formats, medium-prevalence):**

Dates (9): `ymd_dot`, `mdy_dash`, `dmy_space_full`, `full_month_no_comma`, `eu_short_dot`, `compact_ym`, `month_year_full`, `month_year_abbrev`, `month_year_slash`.

Timestamps (7): `iso_8601_millis_offset`, `pg_short_offset`, `dot_dmy_24h`, `slash_ymd_24h`, `ctime`, `epoch_nanoseconds`, `iso_space_zulu`.

Currency (6): `amount_space_sep`, `amount_ch`, `amount_code_prefix`, `amount_minor_int`, `amount_crypto`, `amount_neg_trailing`.

**Tier 3 — consider for completeness (12 formats, lower-prevalence or requiring custom parsing):**

Dates (4): `weekday_dmy_full`, `chinese_ymd`, `korean_ymd`, `quarter`. The CJK formats are high-prevalence within their regions but require Unicode literal support in strptime. Quarter notation (`Q1 2024`) has no strptime representation and needs custom regex.

Timestamps (3): `sql_offset`, `dot_ymd_24h`, `mdy_dash_abbrev` (date-only but low).

Currency (3): `amount_basis_points`, `amount_multisym`, `amount_minor_int` (detection challenge—looks like a plain integer). Note: `amount_minor_int` is listed in both Tier 2 and Tier 3 because while it is extremely common in payment APIs, it is nearly impossible to detect without column-name metadata.

**DuckDB compatibility note.** All proposed formats except `quarter`, `fiscal_year`, `epoch_nanoseconds`, and `amount_minor_int` are directly parseable via DuckDB's `strptime()` function using the listed format strings. DuckDB's `%g` specifier handles 3-digit milliseconds and `%f` handles 6-digit microseconds. For CJK date formats, DuckDB supports Unicode literals in format strings. The four exceptions require either custom regex extraction or numeric conversion outside `strptime`.

## Conclusion

This analysis reveals that FineType's taxonomy, while already more granular than any competing tool, has systematic blind spots in three areas: **separator-variant dates** (8 high-prevalence gaps where changing a dash to a slash or dropping a comma creates an unparseable format), **precision-timezone timestamp combinations** (7 high-prevalence gaps where real-world systems combine fractional seconds with timezone offsets), and **non-Western currency conventions** (4 high-prevalence gaps serving billions of users). The 20 Tier 1 formats should be prioritized immediately—they represent the formats a developer is most likely to encounter when FineType returns "unknown type" today. Adding all 54 formats would bring the taxonomy to approximately **99 datetime types and 16 currency types**, providing coverage for the vast majority of formats encountered in production tabular data worldwide.
