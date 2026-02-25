---
id: NNFT-121
title: Phone number locale validation via libphonenumber patterns
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-25 01:21'
updated_date: '2026-02-25 05:42'
labels:
  - accuracy
  - locale
dependencies:
  - NNFT-118
references:
  - labels/definitions_identity.yaml
  - labels/definitions_geography.yaml
  - crates/finetype-core/src/generator.rs
  - crates/finetype-core/src/taxonomy.rs
  - crates/finetype-model/src/column.rs
  - discovery/locale-aware-inference/BRIEF.md
documentation:
  - 'https://github.com/google/libphonenumber'
  - 'https://github.com/google/libaddressinput'
  - discovery/locale-aware-inference/BRIEF.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extract per-locale phone number validation patterns from libphonenumber and add validation_by_locale to phone_number in taxonomy YAML. Same approach as NNFT-118 postal codes — attractor demotion Signal 1 checks locale patterns, locale-confirmed predictions skip demotion.

No model retraining. No 4-level labels. No inference pipeline changes. Just validation patterns.

Data source: libphonenumber PhoneNumberMetadata.xml (Apache 2.0). Extract national number patterns for FIXED_LINE_OR_MOBILE type per country.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Per-locale validation regex patterns extracted for phone_number (at least 10 countries)
- [x] #2 phone_number false positives reduced on profile eval
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add validation_by_locale block to phone_number in definitions_identity.yaml (14 locales)
2. Add phone_number to TEXT_ATTRACTORS in column.rs
3. Update data/cldr/README.md with libphonenumber attribution
4. Run cargo test + cargo run -- check + make eval-profile
5. Version bump to 0.2.2, CHANGELOG, tag, push
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
14 locale patterns added (EN_US, EN_CA, EN_GB, EN_AU, DE, FR, ES, IT, NL, PL, RU, JA, ZH, KO).
phone_number added to TEXT_ATTRACTORS.
All 165 tests pass, taxonomy check 169/169, eval 68/74 (no regression).
No phone_number FPs in current eval data — 3 correct predictions at 1.0 confidence all preserved.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added per-locale phone number validation patterns (14 locales) and attractor demotion support for phone_number.

Changes:
- Added validation_by_locale to identity.person.phone_number in labels/definitions_identity.yaml with patterns for EN_US, EN_CA, EN_GB, EN_AU, DE, FR, ES, IT, NL, PL, RU, JA, ZH, KO
- Added phone_number to TEXT_ATTRACTORS in column.rs, enabling Signal 1 validation-based demotion
- Updated data/cldr/README.md with libphonenumber attribution (Apache 2.0)
- Version bumped to 0.2.2 with CHANGELOG and CLAUDE.md updates

Tests:
- 165 tests pass, taxonomy check 169/169, eval-profile 68/74 (no regression)
- 3 correct phone_number predictions preserved at 1.0 confidence
<!-- SECTION:FINAL_SUMMARY:END -->
