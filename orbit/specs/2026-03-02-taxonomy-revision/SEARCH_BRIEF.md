# Taxonomy Revision — External Research Brief

**Task:** NNFT-176
**Goal:** Identify the column types that matter most to data analysts and compare against FineType's current 163-type taxonomy to find gaps, over-engineering, and naming issues.

## What FineType Does

FineType classifies text data into semantic types. Given a column of values, it predicts what kind of data it contains (email, date, IP address, etc.). Each type is a **transformation contract** — if FineType says it's `datetime.date.us_slash`, that guarantees `strptime(value, '%m/%d/%Y')` will succeed.

The taxonomy has 163 types across 6 domains. The full list is in Appendix A below.

## Research Questions

### Q1: What column types appear most frequently in real-world tabular datasets?

Search across:
- **Kaggle** — most popular/downloaded datasets across all categories
- **Government open data** — data.gov, data.gov.uk, EU Open Data Portal, Australian data.gov.au
- **Public interest** — WHO, World Bank, UN, OECD statistical databases
- **Enterprise/SaaS** — common CRM, ERP, e-commerce, and analytics exports (Salesforce, Shopify, Stripe, HubSpot schemas)
- **Scientific** — common formats in genomics, climate, social science

For each source, catalogue the **column types you observe** (not FineType labels — natural language descriptions like "percentage", "currency amount with symbol", "file path"). Rank by frequency.

### Q2: What type taxonomies do existing tools and standards use?

Compare FineType's taxonomy against:
- **pandas `infer_dtype`** — what types does pandas recognise?
- **Great Expectations** — what type expectations exist?
- **schema.org** — property types relevant to tabular data
- **Frictionless Data / Table Schema** — field types and formats
- **CSVW (W3C)** — datatype annotations for CSV
- **OpenAI structured outputs / function calling** — what types do they support?
- **dbt** — common data type tests and expectations
- **Airbyte / Fivetran** — connector schema types
- **Excel / Google Sheets** — number format categories that users expect

For each, note types they have that FineType doesn't, and vice versa.

### Q3: What are the most common analyst frustrations with data type detection?

Search for:
- Blog posts, Stack Overflow questions, Reddit threads about data type inference problems
- Common complaints about pandas dtype inference, CSV import type detection
- "I wish my tool could detect..." patterns
- Data cleaning pain points related to type ambiguity

### Q4: What types would enable the highest-value DuckDB transformations?

FineType's value proposition is that each type maps to a DuckDB cast expression. Which types would unlock the most useful transformations?

Think about:
- Types where knowing the format saves analysts significant manual work
- Types where incorrect casting causes silent data corruption
- Types where locale/format ambiguity is a real problem (dates, numbers, currencies)

## What We Already Cover (Appendix A)

### container (11 types)
- **array**: comma_separated, pipe_separated, semicolon_separated, whitespace_separated
- **key_value**: form_data, query_string
- **object**: csv, json, json_array, xml, yaml

### datetime (46 types)
- **component**: century, day_of_month, day_of_week, month_name, month_number, year
- **date**: abbreviated_month, day_first_dash, day_first_dot, day_first_slash, iso_8601, long_full_month, month_first_dash, month_first_dot, month_first_slash, month_year, us_dash, us_dot, us_slash, weekday_abbreviated, weekday_full, year_month, year_only
- **duration**: iso_8601
- **epoch**: epoch_microseconds, epoch_milliseconds, epoch_seconds
- **offset**: timezone_name, utc
- **time**: hh_mm, hh_mm_12h, hh_mm_ss, hh_mm_ss_12h, iso_8601
- **timestamp**: date_hhmm, date_hhmmss, iso_8601, iso_8601_with_offset, rfc_2822, rfc_3339, rfc_3339_nano, sortable, t_separator, us_date_hhmmss, us_date_hhmmss_12h, with_timezone

### geography (16 types)
- **address**: city, country, full_address, postal_code, street_address
- **contact**: phone_number
- **coordinate**: latitude, longitude, pair
- **location**: calling_code, continent, country, region, state
- **transportation**: iata_code, icao_code

### identity (14 person + 14 payment + 3 medical = 31 types)
- **medical**: dea_number, ndc, npi
- **payment**: credit_card_number, currency_code, currency_symbol, cusip, cvv, ean, iban, isbn, isin, issn, lei, sedol, swift_code
- **person**: age, email, first_name, full_name, gender, height, job_title, last_name, middle_name, password, username, weight, entity_name, occupation

### representation (29 types)
- **boolean**: binary, initials, terms
- **code**: hex_color
- **discrete**: categorical, ordinal
- **file**: base64, excel_format, file_extension, mime_type
- **numeric**: decimal_number, float, integer, percentage, si_number, increment
- **scientific**: chemical_formula, scientific_notation, semver, temperature, unit_of_measure
- **text**: alphanumeric_id, entity_name, paragraph, sentence, slug, text, word, json_path

### technology (30 types)
- **code**: doi, regex, sql
- **cryptographic**: hash_md5, hash_sha1, hash_sha256, hash_sha512
- **development**: boolean, cron, http_method, http_status_code, log_level, user_agent
- **hardware**: mac_address, port
- **internet**: domain, email, hostname, ip_v4, ip_v6, uri, url, url_path, url_query, tld, http_header

## Specific Questions About Our Taxonomy

1. **Is `identity.payment` the right home for financial identifiers?** ISIN, CUSIP, SEDOL, LEI are securities identifiers, not payment instruments. Would analysts look for these under "payment"?

2. **Should `representation.scientific` exist as a category?** Temperature, chemical_formula, unit_of_measure — are these useful enough to justify the category, or are they over-engineered?

3. **Is the datetime domain over-specified?** 46 types for dates/times. Do analysts need to distinguish `datetime.date.us_slash` from `datetime.date.month_first_slash`? Or would fewer, broader types be more useful?

4. **What about common business types we're missing?** Think: currency amounts with symbols ($1,234.56), percentages (already have), ratios, scores/ratings, status enums, priority levels.

5. **Duplicate email** — `identity.person.email` and `technology.internet.email` both exist. Is this confusing?

## Deliverable Format

For each finding, provide:
```
Type: [natural language name]
Source: [where you found it — dataset name, tool, standard]
Frequency: [how common — high/medium/low with evidence]
FineType equivalent: [existing type if any, or "MISSING"]
Analyst value: [why an analyst would care about detecting this]
DuckDB transform: [what cast/transform would be useful, if applicable]
```

Group findings into:
1. **High-priority gaps** — common types FineType should add
2. **Removal candidates** — types that cause more harm than good
3. **Rename suggestions** — types with confusing names or wrong categories
4. **Structure changes** — domain/category reorganisation proposals
