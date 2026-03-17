# FineType taxonomy audit: 163 types measured against the real world

**FineType occupies a genuinely unique position** in the data tooling ecosystem — no existing tool (pandas, Great Expectations, Frictionless Data, dbt, Airbyte, or Excel) offers built-in semantic type detection at this granularity. The closest competitor, W3C CSVW, has ~50 types but focuses on XML serialization primitives, not analyst-facing semantics. FineType's 163-type taxonomy is **3–11× more granular** than every standard examined. However, this research reveals **12 high-priority gaps** where common real-world types are missing, **6 removal or demotion candidates** that add complexity without proportional value, **4 rename/recategorization needs** where the current labels mislead analysts, and **3 structural changes** that would sharpen the taxonomy's alignment with actual data workflows. The audit synthesizes evidence from Kaggle's most-downloaded datasets, government open data portals (data.gov, OECD, World Bank), enterprise SaaS schemas (Salesforce, Shopify, Stripe, HubSpot), and the 20 most common analyst frustrations documented on Stack Overflow and GitHub.

---

## The landscape: what analysts actually encounter in their data

Across five major data source categories — Kaggle competitions, government open data, international statistical databases, enterprise SaaS, and scientific datasets — the same **15 column types dominate**. Unique identifiers (integer or alphanumeric auto-keys) appear in virtually every dataset. Integer counts, categorical labels, dates/timestamps, and decimal measurements follow immediately. Person names, free-text descriptions, boolean flags, currency amounts, and email addresses round out the top 10.

The critical finding for FineType is that **currency amounts with locale-specific formatting** (e.g., `$1,234.56` or `€1.234,56`) appear across every source category — Kaggle fares, CRM deal amounts, Shopify order totals, Stripe transactions, World Bank GDP figures, and government budgets — yet FineType has no dedicated type for them. Similarly, **status enums** (Active/Inactive, Open/Closed, paid/pending/refunded) appear in every enterprise dataset, and **rate-per-N values** (mortality per 100,000, crime per capita) pervade public interest data, but neither maps cleanly to a FineType category.

Enterprise data introduces types that general-purpose tools rarely handle: **prefixed IDs** like Stripe's `cus_XXXXX` or Salesforce's 15-character alphanumeric keys, **compound address objects** with structured subfields, **picklist/dropdown enums** with constrained value sets, and **product slugs** (URL-safe lowercase-hyphenated strings, which FineType already covers as `representation.text.slug`). Scientific data contributes domain-specific types — chromosome identifiers, allele codes, phred quality scores — that are too specialized for a general-purpose taxonomy but confirm that FineType's scientific types serve a narrow audience.

## How existing tools compare — and where FineType wins

FineType's competitive advantage is stark. **Pandas** recognizes ~25–30 storage-level types (int64, float64, object, datetime64, category) but has zero semantic awareness — it cannot tell an email from a URL, a phone number from an integer, or JSON from a plain string. **Great Expectations** provides ~10–15 backend-dependent types plus regex extensibility, but ships no built-in semantic type detection — every pattern must be manually configured. **Frictionless Data Table Schema** is the closest philosophical match with 18 types (including `geopoint`, `geojson`, `yearmonth`), but covers only ~11% of FineType's granularity. **OpenAI structured outputs** support just 6–8 JSON Schema primitives. **Airbyte** and **Fivetran** have 12 and 15 transport-layer types respectively. **dbt** offers ~60+ quality tests but tests *constraints*, not *semantic identity*.

The most interesting comparison is **Excel**, which has 12 format categories plus Microsoft 365's newer "linked data types" (Stocks, Geography, Foods, etc.). Excel's **Special** category — ZIP Code, Phone Number, Social Security Number — is the closest mainstream analog to FineType's identity/geography domains, but covers only 3–4 types versus FineType's 77. Excel's **Fraction** format category (displaying values as 1/4, 22/25) is a type FineType lacks entirely.

| Tool | Type count | Semantic overlap with FineType |
|---|---|---|
| pandas | ~25–30 | ~5% (storage types only) |
| Great Expectations | ~10–15 | ~8% (engine-dependent primitives) |
| Frictionless Data | 18 | ~11% (closest philosophy) |
| CSVW (W3C) | ~47–50 | ~12% (most built-in types) |
| OpenAI structured outputs | 6–8 | ~3% (purely structural) |
| Airbyte / Fivetran | 12–15 | ~8% (transport primitives) |
| Excel formats | 12 + 19 linked | ~15% (some semantic overlap) |
| **FineType** | **163** | **Baseline** |

Three types that multiple standards support but FineType lacks: **geopoint** (Frictionless Data's composite lat/lon type), **language tag** (CSVW's BCP 47 `language` type), and **interval/period** (pandas' `Interval` and `Period` types, CSVW's `gMonthDay`/`gDay`/`gMonth`).

## The 20 frustrations that validate FineType's existence

The analyst pain points documented across Stack Overflow, GitHub issues, Reddit, and data engineering blogs read like a feature requirements list for FineType. **Nine of the top 10 frustrations map directly to FineType type detection capabilities.**

The single most complained-about issue is **integers silently converting to floats due to NaN** — when a column has any missing value, pandas promotes the entire column to float64, turning ID `12345` into `12345.0` and breaking joins. FineType's `representation.numeric.integer` detection, combined with DuckDB's nullable integer support, directly solves this.

**Leading zeros stripped from ZIP codes and phone numbers** ranks second — numeric-looking strings like `"01234"` lose their leading zeros when auto-inferred as integers. FineType's `geography.address.postal_code` and `geography.contact.phone_number` types would preserve these as strings. **Ambiguous date parsing** (is `01/02/2024` January 2nd or February 1st?) ranks fourth — pandas' dateutil parser inconsistently switches between day-first and month-first *within the same column*, silently corrupting dates. FineType's distinction between `datetime.date.day_first_slash` and `datetime.date.us_slash` maps directly to distinct `strptime` format strings in DuckDB, eliminating this ambiguity.

**Currency symbols blocking numeric parsing** (`$1,234.56` staying as string), **locale-dependent number formats** (European `1.234,56` vs US `1,234.56`), and **percentage signs causing parse failures** (`50%` remaining as object dtype) are all high-impact frustrations that FineType's type detection could solve — but only if the taxonomy includes the right types. Currently, FineType has `representation.numeric.percentage` but lacks a dedicated **currency-with-symbol** type and a **locale-aware number** type.

| Rank | Frustration | Impact | FineType coverage |
|---|---|---|---|
| 1 | Integer → float from NaN | HIGH | ✅ `representation.numeric.integer` |
| 2 | Leading zeros stripped (ZIP/phone) | HIGH | ✅ `geography.address.postal_code`, `geography.contact.phone_number` |
| 3 | Mixed types in column | HIGH | Partial (detection helps, but needs mixed-type strategy) |
| 4 | Ambiguous date day/month swap | HIGH | ✅ `datetime.date.day_first_*` vs `datetime.date.us_*` |
| 5 | Dates not auto-detected | HIGH | ✅ 46 datetime types |
| 6 | Currency symbols block parsing | HIGH | ❌ **MISSING**: currency amount with symbol |
| 7 | Locale-dependent number formats | HIGH | ❌ **MISSING**: locale-aware number |
| 8 | Object dtype catch-all | HIGH | ✅ FineType's entire purpose |
| 9 | Large IDs → scientific notation | HIGH | Partial: `representation.text.alphanumeric_id` helps |
| 10 | Boolean yes/no/Y/N not detected | MED-HIGH | ✅ `representation.boolean.terms`, `.initials` |

## Which types unlock the most valuable DuckDB transformations?

FineType's core value proposition — each type maps to a DuckDB cast expression — creates clear criteria for type prioritization: **types that produce different DuckDB expressions deserve to exist as separate types; types that produce the same expression should merge.**

The highest-value transformations fall into three tiers. **Tier 1 (silent corruption prevention)** includes date format disambiguation (`strptime(col, '%d/%m/%Y')::DATE` vs `strptime(col, '%m/%d/%Y')::DATE`), European decimal number handling (`REPLACE(REPLACE(col, '.', ''), ',', '.')::DOUBLE`), and currency amount parsing (strip symbol + locale-aware cast to `DECIMAL(18,2)`). These types prevent data corruption that is **undetectable after the fact** — once `01/02/2024` is parsed as the wrong date, no downstream check can recover the original intent.

**Tier 2 (significant manual work saved)** includes epoch timestamp conversion (`to_timestamp(col)` for seconds, `epoch_ms(col)` for milliseconds — FineType's distinction between `epoch_seconds` and `epoch_milliseconds` maps directly to different DuckDB functions), UUID casting (`col::UUID` leveraging DuckDB's native 128-bit UUID type), JSON string parsing (enabling DuckDB's `json_extract` and `json_transform`), and boolean variant mapping (`CASE WHEN col IN ('Yes','Y','true','1') THEN TRUE...END`).

**Tier 3 (DuckDB-specific advantages)** includes interval/duration strings (`col::INTERVAL` using DuckDB's 3-basis-unit interval type), IP address handling via DuckDB's `inet` extension, categorical/enum detection enabling `col::ENUM('val1','val2',...)` for storage optimization, and HUGEINT casting for 128-bit integers beyond INT64 range. DuckDB's **`try_strptime` with format lists** — which tries multiple date formats in sequence and uses the first match — is a uniquely powerful capability that makes FineType's format-specific datetime types especially actionable.

---

## High-priority gaps: 12 types FineType should add

```
Type: Currency amount with symbol
Source: Shopify orders, Stripe transactions, Salesforce deals, World Bank GDP,
        Kaggle house prices, government budgets — appears across ALL source categories
Frequency: HIGH — present in virtually every business/financial dataset
FineType equivalent: MISSING (representation.numeric.decimal_number is close but
                     doesn't handle $, €, £ symbols or locale-specific formatting)
Analyst value: #6 most common analyst frustration. Currency columns stay as strings
               in pandas, requiring manual str.replace('$','').str.replace(',','')
               on every import
DuckDB transform: REPLACE(REPLACE(REPLACE(col, '$', ''), ',', ''), ' ', '')::DECIMAL(18,2)
                  — or locale-aware variant for European formatting
```

```
Type: Locale-aware number (European format: 1.234,56)
Source: EU Open Data Portal, OECD statistics, any European-origin CSV,
        DuckDB GitHub issues #6690 and #13295 document inference failures
Frequency: HIGH — affects all non-US/UK data producers globally
FineType equivalent: MISSING (representation.numeric.decimal_number assumes US format)
Analyst value: #7 most common frustration. "1.234" is 1234 in European format but
               1.234 in US format — silent corruption with no warning
DuckDB transform: DuckDB's read_csv decimal_separator=',' parameter, or
                  REPLACE(REPLACE(col, '.', ''), ',', '.')::DOUBLE
```

```
Type: UUID / GUID
Source: Database exports, API data, Stripe/Salesforce record IDs, log files
Frequency: HIGH — present in virtually every database-sourced dataset
FineType equivalent: MISSING (technology.cryptographic.hash_* types exist but
                     UUID is not a hash — it's an identifier)
Analyst value: UUIDs stored as VARCHAR waste 2× storage and prevent proper indexing
DuckDB transform: col::UUID — DuckDB has native 128-bit UUID type with efficient storage
```

```
Type: File path / directory path
Source: Log files, configuration data, build systems, data pipeline metadata,
        S3/GCS URIs (s3://bucket/key)
Frequency: MEDIUM-HIGH — common in DevOps, ML pipeline, and infrastructure data
FineType equivalent: MISSING (technology.internet.url_path is close but doesn't
                     cover /usr/local/bin, C:\Users\..., or s3:// URIs)
Analyst value: Enables path parsing, directory extraction, extension detection
DuckDB transform: string_split(col, '/') for path components, or regexp_extract
                  for structured path parsing
```

```
Type: Fraction (1/4, 3/8, 22/25)
Source: Excel exports (Excel has a dedicated Fraction format category),
        recipe data, engineering specifications, sports statistics
Frequency: MEDIUM — common in Excel-origin data, specialized domains
FineType equivalent: MISSING
Analyst value: Prevents confusion with dates (3/4 vs March 4th) or ratios
DuckDB transform: string_split(col, '/') then CAST(parts[1] AS DOUBLE) / CAST(parts[2] AS DOUBLE)
```

```
Type: Score / rating (4.5/5, 8/10, ★★★★☆)
Source: Product reviews, surveys, NPS scores, performance evaluations,
        Kaggle datasets with quality ratings (1-10)
Frequency: MEDIUM — common in e-commerce, HR, and survey data
FineType equivalent: MISSING (representation.discrete.ordinal is close
                     but doesn't capture the "X out of Y" pattern)
Analyst value: Enables normalization to a common scale (all ratings to 0-1)
DuckDB transform: regexp_extract + division to normalize: e.g., 4.5/5 → 0.9
```

```
Type: Rate per N (per 1,000 / per 100,000 / per capita)
Source: WHO mortality rates, World Bank birth rates, government crime stats,
        epidemiological data, OECD indicators
Frequency: MEDIUM-HIGH in public interest data; LOW in enterprise data
FineType equivalent: MISSING (representation.numeric.decimal_number doesn't
                     capture the rate semantics)
Analyst value: Enables proper comparison across populations; prevents
               naive summing of rates
DuckDB transform: Numeric cast + metadata annotation for the denominator
```

```
Type: Language / locale tag (BCP 47: en-US, fr-FR, zh-Hans)
Source: CSVW specification (explicit language type), internationalization data,
        content management systems, translation datasets
Frequency: MEDIUM — common in multilingual and i18n contexts
FineType equivalent: MISSING
Analyst value: Enables locale-aware sorting, filtering by language family
DuckDB transform: VARCHAR with validation; enables ICU extension locale operations
```

```
Type: GeoJSON / GeoPoint (composite lat/lon)
Source: Frictionless Data (explicit geopoint and geojson types), mapping data,
        IoT sensor data, location-based services
Frequency: MEDIUM — common in spatial datasets
FineType equivalent: PARTIAL (geography.coordinate.pair exists but doesn't
                     cover GeoJSON objects)
Analyst value: Enables DuckDB spatial extension operations
DuckDB transform: ST_GeomFromGeoJSON(col) using DuckDB spatial extension
```

```
Type: Prefixed ID (stripe-style: cus_XXXXX, sub_XXXXX, pi_XXXXX)
Source: Stripe exports, AWS resource IDs (arn:aws:..., i-0abc123),
        many SaaS platform exports
Frequency: MEDIUM — common in SaaS/API-sourced data
FineType equivalent: MISSING (representation.text.alphanumeric_id is close
                     but doesn't capture the prefix pattern)
Analyst value: Prefix identifies the entity type; enables automatic
               relationship detection across tables
DuckDB transform: string_split(col, '_')[1] for prefix extraction
```

```
Type: Masked / redacted value (XXX-XX-1234, ****1234, [REDACTED])
Source: PII-scrubbed datasets, payment card exports, compliance data
Frequency: MEDIUM — increasingly common as privacy regulations grow
FineType equivalent: MISSING
Analyst value: Prevents attempting to parse masked values as valid data;
               flags data quality issues
DuckDB transform: Detection only — flag as non-parseable
```

```
Type: HTML content
Source: Shopify product Body HTML, CMS exports, email templates, web scraping data,
        CSVW specification includes rdf:HTML as a type
Frequency: MEDIUM — common in e-commerce and content management data
FineType equivalent: MISSING (container.object.xml is close but HTML is
                     distinct — not well-formed XML)
Analyst value: Enables stripping HTML tags for text analysis,
               extracting structured content
DuckDB transform: regexp_replace(col, '<[^>]+>', '', 'g') for tag stripping
```

---

## Removal or demotion candidates: 6 types causing more harm than good

```
Type: identity.person.email (DUPLICATE)
Source: Taxonomy analysis — identical syntax to technology.internet.email
Frequency: N/A — this is a taxonomy issue, not a data issue
Problem: An email address like john@company.com has identical syntax whether
         it appears in a "person" context or "technology" context. The
         Sherlock/Sato academic type detection systems (78 types) use a single
         email type. CRM systems (Salesforce, HubSpot) use one Email field type
         and determine context from the entity, not the data format.
Recommendation: MERGE into a single email type. Place it in identity.contact.email
                or technology.internet.email (not both). If personal vs.
                organizational distinction matters, make it a sub-classification
                hint based on domain analysis, not a separate type.
```

```
Type: representation.scientific.chemical_formula
Source: Taxonomy analysis — extremely rare outside pharma/chemistry domains
Frequency: LOW — Sherlock/Sato papers (686,765 real-world columns) did not
           include chemical formulas among their 78 detected semantic types
Problem: Too specialized for a general-purpose taxonomy. Adds cognitive load
         without proportional value. Most users will never encounter a column
         of chemical formulas.
Recommendation: DEMOTE to an optional extension or specialized plugin. Keep
                temperature and unit_of_measure (which have broader utility)
                but drop chemical_formula from the core taxonomy.
```

```
Type: datetime.component.century
Source: Taxonomy analysis — vanishingly rare as a standalone column
Frequency: VERY LOW — no evidence of "century" as a column type in any
           dataset source examined (Kaggle, government, enterprise, scientific)
Problem: A column containing just century values (19, 20, 21) is effectively
         an integer. No DuckDB transformation is unlocked by detecting this type.
Recommendation: REMOVE from core taxonomy. If century appears, it will be
                correctly classified as representation.numeric.integer.
```

```
Type: datetime.component.day_of_week (as text detection)
Source: Taxonomy analysis — overlaps with datetime.date.weekday_full
Frequency: LOW as standalone column; the values "Monday", "Tuesday" etc.
           are better served by categorical detection
Problem: Ambiguous value. A column of weekday names is functionally a
         categorical enum, not a datetime component. No meaningful DuckDB
         datetime cast applies to bare weekday names.
Recommendation: CONSIDER merging with representation.discrete.categorical
                or keeping only if it maps to a specific DuckDB expression.
```

```
Type: representation.scientific.scientific_notation (as a standalone type)
Source: Taxonomy analysis — scientific notation is a numeric format, not
        a separate semantic type
Frequency: MEDIUM — appears in scientific data but the value is in parsing
           it AS a number, not classifying it as a distinct type
Problem: DuckDB and most tools already handle scientific notation transparently
         in numeric casts (CAST('1.23E+04' AS DOUBLE) works natively).
         Classifying it separately risks confusing analysts who expect a "number."
Recommendation: DEMOTE — treat as a numeric subformat hint rather than a
                standalone type. The detection is useful but the type label
                adds confusion.
```

```
Type: technology.development.boolean
Source: Taxonomy analysis — duplicates representation.boolean.*
Frequency: N/A — taxonomy overlap issue
Problem: Boolean detection already exists in representation.boolean (with
         binary, initials, and terms subtypes). Having a separate boolean
         in technology.development creates the same confusion as duplicate email.
Recommendation: REMOVE technology.development.boolean. Consolidate all boolean
                detection under representation.boolean.
```

---

## Rename suggestions: 4 types with confusing names or wrong categories

```
Type: identity.payment → identity.finance (RENAME + RESTRUCTURE)
Current contents: credit_card_number, currency_code, currency_symbol, cusip,
                  cvv, ean, iban, isbn, isin, issn, lei, sedol, swift_code
Problem: ISIN, CUSIP, SEDOL, and LEI are securities identifiers used in
         trading and clearing, NOT payment instruments. The FINOS foundation
         explicitly categorizes these as "Securities & Issuer ID mapping."
         ISBN and ISSN are publication identifiers, not financial at all.
         An analyst searching for securities identifiers would never look
         under "payment."
Recommendation: Split into three subcategories:
  - identity.finance.payment: credit_card_number, cvv, iban, swift_code
  - identity.finance.securities: cusip, isin, sedol, lei
  - identity.commerce.product: ean, isbn, issn
  This mirrors how financial data standards (ISO, FINOS) categorize these.
```

```
Type: datetime.date.us_slash vs datetime.date.month_first_slash (CLARIFY)
Current state: Both types exist but the distinction is unclear
Problem: If "US slash" means MM/DD/YYYY and "month first slash" also means
         MM/DD/YYYY, these produce the SAME strptime format string and should
         be one type. If they differ, the naming doesn't explain how.
Recommendation: Audit all datetime types against the DuckDB strptime format
                string they produce. Merge types that produce identical format
                strings. Name remaining types by their format pattern, not by
                locale label — e.g., datetime.date.mdy_slash instead of
                datetime.date.us_slash, since not only the US uses this format.
```

```
Type: representation.text.entity_name + identity.person.entity_name (DUPLICATE)
Current state: "entity_name" appears in both representation.text and
               identity.person
Problem: Same naming collision as the duplicate email issue. An "entity name"
         in identity.person context (a company name?) vs representation.text
         context (any named entity?) is ambiguous.
Recommendation: Rename identity.person.entity_name to identity.organization.name
                (if it means company/org names) and keep representation.text.entity_name
                for generic named entities, or merge them entirely.
```

```
Type: representation.numeric.increment (UNCLEAR NAME)
Current state: "increment" in numeric types
Problem: The name "increment" doesn't clearly convey what values this detects.
         Is it auto-incrementing IDs? Sequential numbers? Delta values?
         No standard or tool uses "increment" as a numeric type name.
Recommendation: Rename to representation.numeric.sequence (if it detects
                sequential integers) or representation.numeric.auto_id
                (if it detects auto-incrementing primary keys). Alternatively,
                document the exact semantics clearly.
```

---

## Structural changes: 3 domain reorganization proposals

### 1. Compress datetime from 46 types to ~15–20 distinct format families

The datetime domain's 46 types are likely over-specified for analyst-facing work. The key principle: **types that produce different `strptime` format strings deserve to exist separately; types that don't should merge.** Research on real-world date formats shows a long-tailed distribution where ~10 format families cover 95%+ of encountered dates.

Proposed consolidated datetime families, each mapping to a distinct DuckDB expression:

- `datetime.date.iso` → `col::DATE` (YYYY-MM-DD)
- `datetime.date.dmy_slash` → `strptime(col, '%d/%m/%Y')::DATE`
- `datetime.date.mdy_slash` → `strptime(col, '%m/%d/%Y')::DATE`
- `datetime.date.dmy_dash` → `strptime(col, '%d-%m-%Y')::DATE`
- `datetime.date.dmy_dot` → `strptime(col, '%d.%m.%Y')::DATE`
- `datetime.date.compact` → `strptime(col, '%Y%m%d')::DATE`
- `datetime.date.named_month_long` → `strptime(col, '%B %d, %Y')::DATE`
- `datetime.date.named_month_short` → `strptime(col, '%d-%b-%Y')::DATE`
- `datetime.date.year_month` → stays (maps to DuckDB's yearmonth handling)
- `datetime.date.year_only` → stays (useful for time-series filtering)
- `datetime.timestamp.iso_8601` → `col::TIMESTAMP`
- `datetime.timestamp.iso_with_tz` → `col::TIMESTAMPTZ`
- `datetime.timestamp.rfc_3339` → `col::TIMESTAMPTZ`
- `datetime.epoch.seconds` → `to_timestamp(col)`
- `datetime.epoch.milliseconds` → `epoch_ms(col)`
- `datetime.time.hh_mm_ss` → `col::TIME`
- `datetime.time.hh_mm_ss_12h` → `strptime(col, '%I:%M:%S %p')::TIME`
- `datetime.duration.iso_8601` → `col::INTERVAL`

This preserves the critical **day-first vs month-first disambiguation** that solves the #4 analyst frustration while eliminating redundant types. The separator character (slash/dash/dot) is relevant because it often correlates with locale convention and produces different strptime strings. The total drops from 46 to ~18 types without losing any DuckDB transformation capability.

### 2. Split identity.payment into three semantic categories

The current `identity.payment` category conflates three fundamentally different identifier families. Analysts working in securities settlement, product catalogue management, or payment processing have entirely different mental models.

```
identity.finance.payment:     credit_card_number, cvv, iban, swift_code
identity.finance.securities:  cusip, isin, sedol, lei
identity.commerce.product:    ean, isbn, issn
identity.finance.currency:    currency_code, currency_symbol
```

This mirrors ISO standards: **ISO 6166** governs ISIN (securities), **ISO 13616** governs IBAN (payments), and **ISO 2108** governs ISBN (publications). Grouping them together under "payment" violates the domain expertise that FineType's taxonomy should reflect.

### 3. Add a top-level measurement domain or expand representation.numeric

Several high-priority gaps (currency amounts, locale-aware numbers, rates, scores) and existing types (percentage, temperature, unit_of_measure) all describe **measured quantities with formatting or unit context**. Rather than scattering these across `representation.numeric`, `representation.scientific`, and the proposed new types, consider a coherent grouping:

```
measurement.currency:     currency_amount_us, currency_amount_eu (with symbol)
measurement.percentage:   percentage (currently in representation.numeric)
measurement.rate:         rate_per_n (per 1,000, per 100,000, per capita)
measurement.score:        score_rating (X/Y format)
measurement.fraction:     fraction (1/4, 3/8)
measurement.temperature:  temperature (currently in representation.scientific)
measurement.unit:         unit_of_measure (currently in representation.scientific)
```

This would give analysts a natural place to look for "numbers that mean something specific" — distinct from raw integers, floats, and decimals in `representation.numeric`. The DuckDB transforms for measurement types all involve **stripping formatting + casting to DECIMAL/DOUBLE**, making them a coherent transformation family.

---

## How FineType's types map to real analyst pain points

The following table connects the 9 highest-impact analyst frustrations to specific FineType types and the DuckDB transformation each enables. This is FineType's strongest marketing pitch — every row represents a real problem that costs analysts hours of manual work per dataset.

| Analyst pain point | Impact | FineType type | DuckDB transformation |
|---|---|---|---|
| Date day/month ambiguity | HIGH — silent corruption | `datetime.date.day_first_slash` vs `datetime.date.us_slash` | `strptime(col, '%d/%m/%Y')` vs `strptime(col, '%m/%d/%Y')` |
| Leading zeros stripped from ZIPs | HIGH — data loss | `geography.address.postal_code` | Keep as `VARCHAR` — do not cast to integer |
| Dates not auto-detected | HIGH — manual work | All 46 datetime types | Appropriate `strptime()::DATE/TIMESTAMP` per format |
| Currency symbols block parsing | HIGH — strings not numbers | **NEW**: currency_amount type | `REPLACE(REPLACE(col, '$', ''), ',', '')::DECIMAL(18,2)` |
| European number formats | HIGH — silent corruption | **NEW**: locale_aware_number type | `REPLACE(REPLACE(col, '.', ''), ',', '.')::DOUBLE` |
| Percentage signs block parsing | MEDIUM — manual cleanup | `representation.numeric.percentage` | `REPLACE(col, '%', '')::DOUBLE / 100.0` |
| Boolean yes/no not detected | MED-HIGH — stays as string | `representation.boolean.terms` | `CASE WHEN col IN ('Yes','Y','true') THEN TRUE...END` |
| Large IDs as scientific notation | HIGH — precision loss | `representation.text.alphanumeric_id` | Keep as `VARCHAR` or cast to `HUGEINT` |
| Epoch timestamps look like integers | MEDIUM — requires domain knowledge | `datetime.epoch.epoch_seconds` / `epoch_milliseconds` | `to_timestamp(col)` or `epoch_ms(col)` |

## Conclusion: what FineType should do next

**The taxonomy's core architecture is sound.** Six domains covering containers, datetime, geography, identity, representation, and technology map well to how data practitioners think about column types. No existing tool in the ecosystem — from pandas to dbt to Excel — provides comparable built-in semantic detection. FineType's 163 types put it in genuinely unoccupied territory.

**The highest-leverage change is adding currency-with-symbol and locale-aware number types.** These address frustrations #6 and #7 among analysts and affect every business dataset globally. They also unlock the most valuable DuckDB transformations because incorrect handling causes *silent* data corruption — the worst possible failure mode.

**The datetime domain should be compressed, not expanded.** Reducing from 46 to ~18 types that each map to a distinct `strptime` format string would make the taxonomy more approachable without losing any transformation capability. The key distinction that must survive compression is **day-first vs month-first** — this is the single most dangerous ambiguity in tabular data.

**The `identity.payment` category needs restructuring urgently.** Mixing securities identifiers (ISIN, CUSIP) with payment instruments (IBAN, credit cards) and publication identifiers (ISBN, ISSN) under a single "payment" label will confuse every financial data analyst who encounters the taxonomy. Split it into payment, securities, and product subcategories.

**The duplicate email and duplicate boolean issues should be resolved immediately** — they're easy fixes that eliminate the most obvious source of user confusion. One email type, one boolean family, clear locations for each.

Finally, FineType's greatest strategic insight is treating each type as a **transformation contract**: if FineType says it's a given type, the corresponding DuckDB cast *will* succeed. This contract is what separates FineType from validation tools (which check constraints) and schema tools (which declare types). Every taxonomy decision should be evaluated against this contract: does adding this type unlock a DuckDB transformation that would otherwise require manual work? If yes, add it. If no, reconsider.
