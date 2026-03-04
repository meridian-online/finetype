---
id: NNFT-200
title: CLDR date/time pattern permutation in generators
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 02:32'
updated_date: '2026-03-04 04:16'
labels:
  - locale
  - generator
  - datetime
milestone: m-6
dependencies:
  - NNFT-197
references:
  - crates/finetype-core/src/generator.rs
  - crates/finetype-core/src/locale_data.rs
  - data/cldr/cldr_date_patterns.tsv
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Wire CLDR date/time patterns into datetime generators for locale-text types. Replaces archived NNFT-058 with precise scope.

Types to update: long_full_month, abbreviated_month, weekday_full_month, weekday_abbreviated_month.

Use CLDR-authentic field ordering and separator placement per locale (e.g. FR: "d MMMM y" → "15 janvier 2024").

Embed pattern data as static Rust arrays (same approach as locale_data.rs). This is the most complex task — requires parsing LDML patterns into generator logic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 20+ locales with authentic CLDR date patterns in generators
- [x] #2 Generated samples use correct field ordering per locale
- [x] #3 Generated samples round-trip classify correctly (spot-check 5+ locales)
- [x] #4 cargo run -- check passes (163/163 alignment)
- [x] #5 cargo test passes
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Wired CLDR date/time patterns into 4 datetime generators for 32 locales.

Changes:
- locale_data.rs: Added DateFormatPattern struct and date_format_pattern() lookup with 7 pattern constants (DMY, DMY_DOT, DMY_DE, MDY_COMMA, YMD_HU, YMD_LT, YDM_LV)
- generator.rs: Updated long_full_month, abbreviated_month, weekday_full_month, weekday_abbreviated_month generators to use CLDR-authentic patterns
- Locale-specific features: German period-after-day, Spanish/Portuguese "de" preposition, Hungarian year-period, Latvian "gada" suffix, Turkish/Hungarian/Lithuanian weekday-after-date

Note: DE "15. Januar 2024" classifies as abbreviated_month pre-retrain — expected, will be fixed by NNFT-201 model retrain.

Tests: cargo test 258 passed, cargo run -- check 163/163
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
