---
id: NNFT-198
title: Expand postal_code generator to match validation locales
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 02:32'
updated_date: '2026-03-04 04:16'
labels:
  - locale
  - generator
  - geography
milestone: m-6
dependencies:
  - NNFT-195
references:
  - crates/finetype-core/src/generator.rs
  - crates/finetype-core/src/locale_data.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extend gen_postal_code() and postal_format() in generator.rs for all 50+ locales from NNFT-195.

Each locale needs: format identifier + generation branch (2-5 lines each). Generated codes must pass their corresponding validation_by_locale regex.

Approach: Stay hardcoded (Option A from plan). 50 locales is ~150 lines added — manageable without a data-driven approach.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All 50+ locales generate valid postal codes
- [x] #2 Generated codes pass their corresponding validation_by_locale regex
- [x] #3 Generator↔definition alignment passes (cargo run -- check)
- [x] #4 cargo test passes
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded postal_code generator from 13 to 65 locales to match NNFT-195 validation coverage.

Changes:
- locale_data.rs: postal_format() expanded with 51 new locale entries using 18 format identifiers
- generator.rs: gen_postal_code() expanded with 18 new match arms covering all unique postal formats
- Formats include: Czech 3+2, Portuguese XXXX-XXX, Brazilian XXXXX-XXX, Lithuanian LT-prefix, Latvian LV-prefix, Argentine letter+digits+letters, Peruvian LIMA/CALLAO, Maltese AAA+digits, Irish Eircode, Taiwanese 3/5/6 digit, Israeli 5/7 digit, Icelandic 3-digit

Tests: cargo test 258 passed, cargo run -- check 163/163 (8150/8150 samples)
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
