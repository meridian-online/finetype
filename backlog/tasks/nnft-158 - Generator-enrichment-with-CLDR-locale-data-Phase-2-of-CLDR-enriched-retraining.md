---
id: NNFT-158
title: >-
  Generator enrichment with CLDR locale data (Phase 2 of CLDR-enriched
  retraining)
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-27 22:53'
updated_date: '2026-02-27 23:03'
labels:
  - accuracy
  - cldr
  - phase-2
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 2 of the CLDR-Enriched Model Retraining plan (Option C).

Expand locale data tables and datetime generators to use CLDR patterns, producing more diverse training samples. No new types — same 171 types with richer per-type variation.

Key changes:
1. locale_data.rs — Expand month/weekday name tables from 12 → 20+ locales
2. generator.rs — Add locale-aware date format variations (DMY vs MDY ordering, separator variations)
3. Validation patterns — Where CLDR formats exceed current regex, add relaxed patterns in validation_by_locale only

Target locales to add: SV, DA, NO, CS, PT, PT_BR, FI, HU, TR, RO, EL, UK, HR, BG, SK, LT, LV, ET, SL
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 locale_data.rs month_names expanded from 12 to 20+ locales with CLDR-sourced data
- [x] #2 locale_data.rs weekday_names expanded from 12 to 20+ locales with CLDR-sourced data
- [x] #3 locale_data.rs month_abbreviations expanded to match new locales
- [x] #4 locale_data.rs weekday_abbreviations expanded to match new locales
- [x] #5 Date generators produce locale-appropriate format variations (DMY vs MDY vs YMD ordering)
- [x] #6 cargo run -- check passes: all 171 generators aligned with validation
- [x] #7 cargo test passes with no regressions
- [x] #8 No new types added — label space remains at 171
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Generator enrichment with CLDR locale data: expanded from 12 to 31 locale keys for more diverse training samples.

## What changed

### locale_data.rs — 19 new locales

Expanded all four locale data functions (month_names, month_abbreviations, weekday_names, weekday_abbreviations) with CLDR-sourced data for: BG, CS, DA, EL, ET, FI, HR, HU, LT, LV, NO, PT|PT_BR, RO, SK, SL, SV, TR, UK.

Data sourced from CLDR stand-alone (nominative) context — consistent with existing convention. Inflected languages (CS, FI, HR, etc.) use nominative forms, not genitive forms that CLDR provides in "format" context.

### generator.rs — Locale-aware date ordering

- Added `DateOrder` enum (Mdy, Dmy, Ymd) with `date_order()` method
- `abbreviated_month` and `long_full_month` generators now produce locale-appropriate ordering:
  - EN/EN_US/EN_CA → "Aug 25, 2029" (MDY)
  - HU/LT/LV → "2023. június 15." (YMD)
  - All others → "4 juin 2022" (DMY)
- Weekday generators kept at fixed DMY (validation patterns require `Weekday, DD Month YYYY`)
- `generate_all()` cycles through available locales for locale-specific types, giving diverse training data with unchanged 3-level labels

### No type changes

Label space remains 171 types. Same 3-level taxonomy labels. Diversity comes from expanded locale coverage in training data, not label expansion.

## Key design decisions

- Stand-alone (nominative) month forms over genitive — matches existing PL convention
- Weekday dates kept DMY-only — validation pattern `%A, %d %b %Y` is format-fixed
- Locale cycling in generate_all() — each sample rotates through available locales

## Tests

- cargo test: 319 passed (7+98+214), 0 failed
- cargo run -- check: 171/171 generators pass, 100% alignment
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
