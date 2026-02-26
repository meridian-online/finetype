---
id: NNFT-141
title: Expand locale validation coverage using old repo's 36-locale map
status: To Do
assignee: []
created_date: '2026-02-26 00:28'
labels:
  - locale
  - taxonomy
  - precision
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The old finetype prototype (hughcameron/finetype) supported 36 Mimesis locales for locale-specific types. The current system has validation_by_locale patterns for 15 phone locales and 14 postal code locales. Use the old repo's locale list (AR_AE, CS, DA, DE, DE_AT, DE_CH, EL, EN, EN_AU, EN_CA, EN_GB, ES, ES_MX, ET, FA, FI, FR, HU, HR, IS, IT, JA, KK, KO, NL, NL_BE, NO, PL, PT, PT_BR, RU, SK, SV, TR, UK, ZH) as a roadmap for expanding validation_by_locale to more types and more locales.

Each new locale pattern is a measurable, testable improvement per the Precision Principle. Priority types for expansion: addresses, date formats with month names, calling codes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Audit current locale coverage gaps against the 36-locale target list
- [ ] #2 Add validation_by_locale patterns for at least 3 additional types beyond phone_number and postal_code
- [ ] #3 Each new pattern has a test verifying it matches expected formats and rejects non-matches
- [ ] #4 Document the locale expansion roadmap in a discovery brief
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
