---
id: NNFT-223
title: NNFT-224 — Add 54 type definitions to datetime.yaml and finance.yaml
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-05 01:56'
updated_date: '2026-03-05 02:48'
labels:
  - format-coverage
  - taxonomy
  - yaml-definitions
  - workstream-a
dependencies:
  - NNFT-222
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create YAML definitions for all 54 new format types across two domain files.

**Scope: 41 new definitions for datetime domain**
- 25 date format types: dmy_slash, ymd_slash, dmy_dash, mdy_dash, dmy_space_abbrev, dmy_dash_abbrev, dmy_dash_abbrev_short, abbrev_month_no_comma, us_short_slash, eu_short_slash, eu_short_dot, year_month, compact_ym, weekday_dmy_full, chinese_ymd, korean_ymd, jp_era_short, jp_era_long, fiscal_year, and 6 more regional variants
- 16 timestamp format types: iso_8601_milliseconds, iso_8601_milliseconds_offset, iso_8601_micros_offset, sql_microseconds, sql_milliseconds, sql_microseconds_offset, sql_milliseconds_offset, clf (Apache Common Log Format), syslog_bsd, pg_short_offset, slash_ymd_24h, ctime, dot_ymd_24h, and 2 more

Each definition includes:
- title: Human-readable format name
- description: Use case and prevalence context
- broad_type: DuckDB type (DATE, TIMESTAMP, etc.)
- format_string: strptime format (or null for custom parsing like Japanese era)
- transform: SQL expression for DuckDB strptime conversion
- validation: JSON Schema pattern for validation
- tier: [VARCHAR] designation
- samples: 3-5 realistic examples

**Scope: 13 new definitions for finance.currency category**
- 12 currency format types: amount_accounting_us (parentheses), amount_eu_suffix (€ 1.234,56), amount_indian (₹12,34,567.89), amount_nodecimal (1234€), amount_code_prefix (USD 1,234.56), amount_minor_int (123456 cents), amount_crypto (0.025 BTC), amount_basis_points (125 bps), amount_multisym (R$ 1.234,56), amount_space_sep (1 234,56 €), amount_neg_trailing (1,234.56-), yield (+2.5%)
- Each includes samples matching real financial CSV headers

**Japanese era handling (custom parsing required)**:
- JP_ERA_SHORT (R6/01/15): Reiwa 6 = 2024
- JP_ERA_LONG (Reiwa 6/01/15): Full era name
- Requires era offset table: R→2019, H→1989, S→1926, T→1912, M→1868 (+ offset -1 for first year of era)

**CJK Unicode validation**:
- Chinese: Use Unicode character 年 (year), 月 (month), 日 (day)
- Korean: Use Unicode character 년 (year), 월 (month), 일 (day)
- Japanese: Reiwa (令和), Heisei (平成), Showa (昭和), Taisho (大正), Meiji (明治)

**Deliverable**: Updated labels/definitions_datetime.yaml (~1000+ lines, from ~400) and labels/definitions_finance.yaml (~600+ lines, from ~200)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All 41 datetime type definitions added with title, description, broad_type, format_string, transform, validation, tier, samples fields
- [x] #2 All 13 finance.currency type definitions added (12 currency formats + yield)
- [x] #3 Japanese era offset table documented in dmy_dash_abbrev and jp_era_* definitions
- [x] #4 CJK Unicode characters correctly used in format_string (年月日, 년월일)
- [x] #5 Zero validation collisions with existing 163 types (patterns don't overlap ambiguously)
- [x] #6 YAML syntax valid (can be parsed by cargo run -- check)
- [x] #7 All 54 types follow existing YAML structure and naming convention
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan for NNFT-223: YAML Type Definitions

### Phase: Parallel Definition Creation (14-18 hours)

**Objective**: Add 54 complete type definitions across two YAML files with zero validation collisions.

### Workstream Structure

**Substream 1: DateTime Domain (41 new types)** — 9-11 hours
1. **Analyze existing datetime structure** (~1 hr)
   - Read current `labels/definitions_datetime.yaml` (45 existing types)
   - Document field structure: title, description, broad_type, format_string, transform, validation, tier, samples
   - Map existing datetime categories to understand structure

2. **Create 25 date format definitions** (~4 hrs)
   - Standard separator variants (slash, dash, dot, space)
   - Regional variants (DMY, YMD, MDY orderings)
   - CJK formats with Unicode literals
   - Partial dates (year-month, month-day)
   - Each: realistic samples (3-5), validation pattern, strptime format_string, SQL transform

3. **Create 16 timestamp format definitions** (~4 hrs)
   - ISO 8601 variants (milliseconds, microseconds, with/without offset)
   - SQL formats (microseconds, milliseconds, offset variants)
   - Log formats (Apache CLF, syslog BSD, ctime, PostgreSQL)
   - Each: samples, strptime format_string (or null for custom), validation regex, SQL transform

4. **Validate against existing 45 temporal types** (~1 hr)
   - Ensure no validation pattern overlaps that would cause ambiguity
   - Cross-check against existing timestamp/date definitions
   - Document any disambiguation heuristics needed in column.rs

**Substream 2: Finance Domain (13 new types)** — 5-7 hours
1. **Analyze existing finance structure** (~1 hr)
   - Read current `labels/definitions_finance.yaml` (4 currency types)
   - Document field structure and sampling conventions

2. **Create 12 currency format definitions** (~4 hrs)
   - Accounting parentheses, EU suffix, Indian lakh/crore, Swiss apostrophe
   - Zero-decimal, code prefix, minor units, crypto
   - Basis points, multi-symbol, space-separated, negative trailing, yield
   - Each: realistic samples (3-5 amounts), validation pattern, transform, broad_type

3. **Validate against existing 4 currency types + all 163 total types** (~1 hr)
   - Ensure no ambiguity with decimal_number, percentage, or other numeric types
   - Document edge cases (basis points near percentage, zero-decimal similar to plain integer)

### Key Implementation Details

**CJK Unicode Handling** (critical for AC #4):
- Chinese: Use literal characters 年 (U+5E74), 月 (U+6708), 日 (U+65E5)
  - format_string: `"%Y年%-m月%-d日"`
  - samples: `"2024年1月15日"`, `"2023年12月25日"`
- Korean: Use literal characters 년 (U+B144), 월 (U+C6D4), 일 (U+C77C)
  - format_string: `"%Y년 %-m월 %-d일"`
  - samples: `"2024년 1월 15일"`, `"2023년 12월 25일"`
- Japanese era: Custom parsing (format_string: null)
  - Requires era offset table (hardcoded in generator, documented here)
  - Reiwa (令和) R → 2019 offset
  - Heisei (平成) H → 1989 offset
  - Showa (昭和) S → 1926 offset
  - Taisho (大正) T → 1912 offset
  - Meiji (明治) M → 1868 offset

**Validation Pattern Strategy**:
- Universal types: Single validation pattern (apply globally)
- Format-specific: Patterns distinguish from similar types (MDY vs DMY via day >12 rule)
- CJK formats: Unicode character patterns confirm (年, 월, etc.)
- Currency: Symbol + grouping patterns (€, ₹, CHF, etc.)

**Samples Strategy**:
- Realistic values from actual CSVs (research findings)
- Mix of edge cases (first/last day, leap years where relevant)
- For currencies: amounts spanning range (0.01 to 999,999.99)
- For CJK: modern dates + historical dates where applicable

### Serial Gate: Validation Check (~1 hr)
After both substreams complete:
1. Verify YAML syntax validity: `cargo run -- check` (should report 213 types, zero errors)
2. Cross-check validation patterns for ambiguity
3. Document any edge cases in task notes for generator-specialist

### Success Metrics
- ✅ All 54 type definitions present in YAML files with complete field structure
- ✅ CJK Unicode literals correctly placed in format_string
- ✅ Zero validation collisions detected by `cargo run -- check`
- ✅ All 7 acceptance criteria checked off

### Parallel Dependencies
- **Unblocked by**: NNFT-224 (generators can start immediately on task descriptions)
- **Blocks**: NNFT-225 (LabelCategoryMap update needs finalized type names)
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Progress — YAML Definitions Complete

- Datetime: 41 new types (16 timestamp, 23 date, 2 period). File: 1601→2864 lines. YAML valid.
- Finance: 13 new types (11 currency, 2 rate). File: 579→1009 lines. YAML valid.
- Total taxonomy: 163→217 types (54 new, 33% increase).
- CJK Unicode: chinese_ymd (年月日), korean_ymd (년월일), jp_era_short/long (R/H/S/T/M + kanji).
- Plan arithmetic error: correct total is 217, not 213.
- Awaiting NNFT-224 (generators) for cargo run -- check to pass.

## Final Validation — All Checks Pass

- `cargo run --release -- check`: 216/216 generators found, 216/216 fully passing, 10800/10800 samples passed (100%)
- `cargo test --all`: 480 tests passed, 0 failures
- Corrected count: 163 + 53 = 216 types (sql_offset removed as duplicate of rfc_3339)
- All 7 domains green: container (12), datetime (85), finance (29), geography (15), identity (19), representation (32), technology (24)
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 53 new type definitions to FineType taxonomy, expanding from 163 to 216 types (33% increase).

## Changes

**DateTime domain** (`labels/definitions_datetime.yaml`): 40 new types across 3 categories
- **15 timestamp formats**: ISO 8601 milliseconds/microseconds (JavaScript ecosystem), Apache CLF (web logs), syslog BSD (RFC 3164), SQL microseconds/milliseconds with offsets, PostgreSQL short offset, ctime, dot/slash-separated 24h formats
- **23 date formats**: Separator variants (slash/dash/dot/space), partial dates (year_month, compact_ym), CJK Unicode dates (Chinese 年月日, Korean 년월일), Japanese era dates (short R6/01/15 + long Reiwa 6/01/15), regional variants (EU short, US short, DMY/MDY/YMD orderings)
- **2 period types**: New `datetime.period` category with `quarter` (Q1 2024) and `fiscal_year` (FY2024)
- File grew from 1601 to 2864 lines

**Finance domain** (`labels/definitions_finance.yaml`): 13 new types across 2 categories
- **11 currency formats**: Accounting parentheses (US), EU suffix notation, Indian lakh/crore grouping, Swiss apostrophe, zero-decimal (JPY/KRW), ISO code prefix, minor units (cents), crypto notation, multi-symbol (R$, HK$, kr), space-separated, trailing minus
- **2 rate types**: New `finance.rate` category with `basis_points` (125 bps) and `yield` (+2.5%)
- File grew from 579 to 1009 lines

**Generator implementation** (`crates/finetype-core/src/generator.rs`): All 53 generators implemented by teammate (NNFT-224), producing realistic samples that pass 100% validation.

**CLAUDE.md**: Updated taxonomy counts (163→216), domain breakdowns, and in-progress section.

## Scope adjustment
- Original plan: 54 types. Removed `sql_offset` (duplicate of existing `rfc_3339`). Final: 53 new = 216 total.

## Verification
- `cargo run --release -- check`: 216/216 generators, 10800/10800 samples (100%)
- `cargo test --all`: 480 tests passed, 0 failures
- All 7 domains green

## What's next
- NNFT-225: Update LabelCategoryMap for 216 types
- NNFT-226: Retrain CharCNN model on expanded taxonomy
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
