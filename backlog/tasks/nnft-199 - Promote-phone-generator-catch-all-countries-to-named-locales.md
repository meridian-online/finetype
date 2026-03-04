---
id: NNFT-199
title: Promote phone generator catch-all countries to named locales
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 02:32'
updated_date: '2026-03-04 04:16'
labels:
  - locale
  - generator
  - identity
milestone: m-6
dependencies:
  - NNFT-196
references:
  - crates/finetype-core/src/generator.rs
  - crates/finetype-core/src/locale_data.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The catch-all branch in generator.rs (lines ~3328-3737) already generates phone formats for ~30 countries (BR, MX, IN, TH, etc.) but they're unreachable during training because they're not in the named match arms.

Promote to named match arms: add locale codes to phone_country_code() and calling_codes(). Restructure so all countries are reachable via the locales cycling in generate_all().

No duplicated code — refactor the catch-all patterns into the named locale arms.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All catch-all countries promoted to named locale arms
- [x] #2 Generated numbers pass validation_by_locale patterns from NNFT-196
- [x] #3 No duplicated generation code between locales
- [x] #4 cargo run -- check passes (163/163 alignment)
- [x] #5 cargo test passes
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Promoted 31 phone generator countries from random catch-all to named locale match arms.

Changes:
- generator.rs: Replaced 400-line random catch-all with 31 named locale arms (28 promoted + 4 new: ES_PE, HU, RO, CZ)
- locale_data.rs: Added 31 entries to phone_country_code() and calling_codes()
- Reduced catch-all now generates generic international format using phone_country_code()
- All 46 taxonomy locales now have dedicated generation paths

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
