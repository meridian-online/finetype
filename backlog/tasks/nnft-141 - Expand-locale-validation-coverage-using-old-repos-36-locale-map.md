---
id: NNFT-141
title: Expand locale validation coverage using old repo's 36-locale map
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-26 00:28'
updated_date: '2026-02-26 02:00'
labels:
  - locale
  - taxonomy
  - precision
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The old finetype prototype (hughcameron/finetype) supported 36 Mimesis locales for locale-specific types. The current system has validation_by_locale patterns for 15 phone locales and 14 postal code locales. Use the old repo's locale list (AR_AE, CS, DA, DE, DE_AT, DE_CH, EL, EN, EN_AU, EN_CA, EN_GB, ES, ES_MX, ET, FA, FI, FR, HU, HR, IS, IT, JA, KK, KO, NL, NL_BE, NO, PL, PT, PT_BR, RU, SK, SV, TR, UK, ZH) as a roadmap for expanding validation_by_locale to more types and more locales.

Each new locale pattern is a measurable, testable improvement per the Precision Principle. Priority types for expansion: addresses, date formats with month names, calling codes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Audit current locale coverage gaps against the 36-locale target list
- [x] #2 Add validation_by_locale patterns for at least 3 additional types beyond phone_number and postal_code
- [x] #3 Each new pattern has a test verifying it matches expected formats and rejects non-matches
- [x] #4 Document the locale expansion roadmap in a discovery brief
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

Expand validation_by_locale to 3 additional types. All patterns are finite enumerations or well-defined formats from authoritative sources.

### Type 1: calling_code (locale-specific calling codes per country)
Source: ITU-T E.164 country calling codes (public domain)
Approach: Per-locale regex matching the specific country code(s) for each locale
- EN_US/EN_CA: ^\+?1$
- EN_GB: ^\+?44$
- DE: ^\+?49$
- FR: ^\+?33$
- etc. for all 16 locales
Rationale: Currently only universal ^\+?[0-9]{1,4}$ — confirms 90%+ of random digit strings. Per-locale patterns are precise.

### Type 2: month_name (full month names per language)
Source: CLDR / Unicode CLDR JSON (public domain)
Approach: Per-locale enum lists of 12 month names
- EN: [January, February, ...]
- FR: [janvier, février, ...]
- DE: [Januar, Februar, ...]
- etc. for 6 listed locales
Rationale: Currently only EN enum in universal validation. Locale detection enables identifying French vs German months.

### Type 3: day_of_week (weekday names per language)
Source: CLDR / Unicode CLDR JSON (public domain)
Approach: Per-locale enum lists of 7 day names
- EN: [Monday, Tuesday, ...]
- FR: [lundi, mardi, ...]
- DE: [Montag, Dienstag, ...]
- etc. for 6 listed locales
Rationale: Same as month_name — currently EN-only enum.

### Steps
1. Add validation_by_locale to calling_code in definitions_geography.yaml
2. Add validation_by_locale to month_name in definitions_datetime.yaml
3. Add validation_by_locale to day_of_week in definitions_datetime.yaml
4. Run cargo test + cargo run -- check to verify taxonomy alignment
5. Add unit tests for locale detection on each new type
6. Run profile eval to confirm no regression
7. Document locale expansion in discovery brief
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded locale validation coverage from 2 to 5 types (NNFT-141)

Added validation_by_locale patterns for 3 new types:

1. **calling_code** (17 locales) — Per-country calling code validation from ITU-T E.164. Each locale pattern matches the exact calling code for that country (e.g., EN_GB → ^\+?44$, DE → ^\+?49$). Replaces the permissive universal ^\+?[0-9]{1,4}$ for locale detection.

2. **month_name** (6 locales: EN, FR, DE, ES, IT, PT) — Full month name enumerations per language from Unicode CLDR. Enables detecting French months (janvier, février...) vs German (Januar, Februar...) etc.

3. **day_of_week** (6 locales: EN, FR, DE, ES, IT, PT) — Full weekday name enumerations per language from Unicode CLDR.

Also fixed locale re-detection on header hint override: when a header hint changes the predicted label (e.g., first_name → month_name), locale detection now re-runs for the new type instead of clearing to None. This was essential for month_name/day_of_week which the model doesn't recognize in non-English but header hints resolve correctly.\n\nFiles changed:\n- labels/definitions_geography.yaml — calling_code validation_by_locale\n- labels/definitions_datetime.yaml — month_name and day_of_week validation_by_locale\n- crates/finetype-model/src/column.rs — locale re-detection fix + 5 new tests\n- data/cldr/README.md — documented CLDR and ITU-T data sources\n- docs/LOCALE_DETECTION_ARCHITECTURE.md — updated coverage table\n- CLAUDE.md — updated decided items 13 and 16\n\nTests: 202 pass (5 new), clippy clean, taxonomy check clean\nProfile eval: 70/74 (no regression)
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
