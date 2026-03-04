---
id: NNFT-196
title: Expand phone_number validation to 40+ locales
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 02:32'
updated_date: '2026-03-04 03:08'
labels:
  - locale
  - validation
  - identity
milestone: m-6
dependencies: []
references:
  - labels/definitions_identity.yaml
  - 'https://github.com/google/libphonenumber'
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add validation_by_locale entries to definitions_identity.yaml for 25+ new countries, bringing phone_number to 40+ total locales.

Source: Google libphonenumber nationalNumberPattern metadata.

Patterns must include: national format, optional international prefix, extension suffix — consistent with existing pattern structure in the file.

Also update calling_code validation_by_locale to match the new locales.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 40+ locales in phone_number validation_by_locale
- [x] #2 Patterns validate realistic phone formats for each locale (spot-check 3+ locales)
- [x] #3 calling_code validation_by_locale updated to match
- [x] #4 cargo run -- check passes (163/163 alignment)
- [x] #5 cargo test passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Agent completed YAML expansion: 31 new locales added to phone_number (46 total) and 30 new calling_code locales (47 total). All patterns follow existing conventions. cargo test passed (98 core, 258 full). Pending: cherry-pick to main and final verification.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded phone_number validation_by_locale from 15 to 46 locales in definitions_identity.yaml, and calling_code from 16 to 47 locales in definitions_geography.yaml.

Changes:
- Added 31 new phone_number locale validation entries with patterns following existing conventions (optional international prefix, trunk prefix notation, separator class, extension suffix)
- Added 30 new calling_code locale entries
- Covers: Latin America (PT_BR, ES_MX, ES_AR, ES_CL, ES_CO, ES_PE), Europe (SE, NO, DK, FI, CZ, HU, RO, GR, TR, PT, AT, CH, BE, IE), Middle East (AR, IL), Asia-Pacific (HI, TH, MY, SG, PH, ID, TW, NZ), Africa (NG)

Tests: cargo test (258 passed), cargo run -- check (163/163). test_all_taxonomy_schemas_compile and test_all_taxonomy_locale_schemas_compile validated all regex patterns."
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
