---
id: NNFT-195
title: Expand postal_code validation to 50+ locales
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 02:32'
updated_date: '2026-03-04 03:08'
labels:
  - locale
  - validation
  - geography
milestone: m-6
dependencies: []
references:
  - labels/definitions_geography.yaml
  - 'https://github.com/google/libaddressinput'
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add validation_by_locale entries to definitions_geography.yaml for 36+ new countries, bringing postal_code to 50+ total locales.

Source: Google libaddressinput regex patterns (Apache 2.0 licensed).

Target additions: BR, MX, IN, SE, NO, DK, FI, CH, AT, BE, PT, TR, IL, GR, ZA, NG, TH, MY, SG, PH, ID, TW, HK, NZ, IE, CZ, HU, RO, BG, HR, SK, SI, LT, LV, EE, AR.

Update the locales field to include all new locale codes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 50+ locales in postal_code validation_by_locale
- [x] #2 Each pattern rejects non-matching strings (spot-check 3+ locales)
- [x] #3 cargo run -- check passes (163/163 alignment)
- [x] #4 cargo test passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Agent completed YAML expansion: 51 new locales added (65 total). All patterns sourced from Google libaddressinput. Positive/negative testing done. All tests pass. Pending: cherry-pick to main and final verification.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded postal_code validation_by_locale from 14 to 65 locales in definitions_geography.yaml.

Changes:
- Added 51 new locale validation entries with regex patterns sourced from Google libaddressinput (Apache 2.0)
- Covers all major regions: Nordic (SE, NO, DK, FI, IS), Eastern Europe (CZ, HU, RO, BG, HR, SK, SL, SR, UA), Baltics (LT, LV, ET), Middle East (TR, HE, EL, AR_SA, AR_EG, AR_MA), Latin America (ES_MX, ES_AR, ES_CL, ES_CO, ES_PE, PT_BR), Asia (HI, TH, MS, ID, VI, ZH_TW), Africa (ZA, NG, EN_KE)
- All patterns anchored with ^ and $ with correct minLength/maxLength constraints
- Updated locales array from 15 to 66 entries

Tests: cargo test (258 passed), cargo run -- check (163/163). Agent also ran positive/negative regex validation tests."
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
