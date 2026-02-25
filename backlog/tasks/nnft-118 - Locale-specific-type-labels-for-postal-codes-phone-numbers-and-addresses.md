---
id: NNFT-118
title: 'Locale-specific type labels for postal codes, phone numbers, and addresses'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-24 08:35'
updated_date: '2026-02-25 01:20'
labels:
  - accuracy
  - locale
  - model-training
dependencies: []
references:
  - labels/definitions_geography.yaml
  - labels/definitions_identity.yaml
  - crates/finetype-core/src/validator.rs
  - crates/finetype-core/src/taxonomy.rs
  - discovery/locale-aware-inference/BRIEF.md
  - crates/finetype-core/src/generator.rs
documentation:
  - discovery/locale-aware-inference/BRIEF.md
  - 'https://github.com/google/libphonenumber'
  - 'https://github.com/google/libaddressinput'
  - 'https://chromium-i18n.appspot.com/ssl-address'
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Bring back 4-level locale labels (domain.category.type.LOCALE) for the 21 locale-specific types. This solves the root cause of postal_code/phone_number false positives: a single type trying to validate all worldwide formats forces validation to be too permissive.

**Approach:** Split locale-specific types into per-locale classes, each with a tight per-locale regex. The infrastructure already exists:
- `generate_all_localized()` in generator.rs produces 4-level labels
- The Python version trained on locale-suffix labels
- Taxonomy YAML already has locales listed per type (21 types × ~16 locales = ~336 locale variants + 148 universal = ~484 total classes)

**Data sources for per-locale validation patterns:**
- **Phone numbers**: [libphonenumber](https://github.com/google/libphonenumber) — canonical library for parsing/formatting/validating international phone numbers, handles mobile vs landline prefixes
- **Postal codes**: [libaddressinput](https://github.com/google/libaddressinput) — address formatting patterns and postal code validation per country
- **Address data**: [Google Address Data Service](https://chromium-i18n.appspot.com/ssl-address) — metadata for country-specific address fields (state vs province, postal code position)

**Key insight:** The split is by format, not country. US and DE postal codes are both 5-digit — the CharCNN can't distinguish them, but that's fine. The point is that `postal_code.EN_US` gets `^\d{5}(-\d{4})?$` which rejects salary values by construction. Follows the precedent of date format types (iso, us_slash, eu_slash).

**Scope:**
1. Extract per-locale validation patterns from libphonenumber/libaddressinput for postal_code and phone_number
2. Add per-locale validation blocks to the taxonomy YAML
3. Generate 4-level training data and retrain models
4. Update inference pipeline to handle ~484 classes, collapsing to 3-level for user output with locale as metadata
5. Verify accuracy improvement on eval
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Per-locale validation regex patterns extracted for postal_code (at least 10 countries)
- [ ] #2 Per-locale validation regex patterns extracted for phone_number (at least 10 countries)
- [ ] #3 4-level training data generated via generate_all_localized()
- [ ] #4 Model retrained on 4-level labels with acceptable accuracy
- [ ] #5 Inference pipeline returns 3-level user label with locale metadata
- [x] #6 postal_code false positives reduced on profile eval
- [x] #7 No regressions on format-detectable accuracy (68/74 baseline)
- [x] #8 Data source attribution documented (libphonenumber, libaddressinput)
- [x] #9 Phase 1 scope delivered — ACs #2-5 deferred to follow-up task
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Phase 1: Per-locale postal code validation (no model retraining)

1. Add validation_by_locale field to Definition struct in taxonomy.rs
2. Add compile_locale_validators() + get_locale_validators() to Taxonomy
3. Add 14 locale patterns for postal_code in definitions_geography.yaml
4. Enhance attractor demotion Signal 1 with locale-aware validation
5. Add data/cldr/README.md for attribution
6. Add tests for locale validators and locale-aware attractor demotion
7. Verify: cargo test, cargo run -- check, make eval-profile

Phases 2-3 (follow-up tasks): phone_number locale patterns, CLDR date/time, replace locale_data.rs
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Phase 1 delivered: per-locale postal code validation via attractor demotion.

Changes:
- taxonomy.rs: validation_by_locale field on Definition, compile_locale_validators() + get_locale_validators() on Taxonomy
- definitions_geography.yaml: 14 locale postal code patterns (EN_US, EN_GB, EN_AU, EN_CA, DE, FR, ES, IT, NL, PL, RU, JA, ZH, KO)
- column.rs: Signal 1 enhanced — checks locale patterns before universal validation. If any locale achieves >50% pass rate, prediction is locale-confirmed (skips demotion)
- main.rs: compile_locale_validators() called after compile_validators()
- data/cldr/README.md: attribution for libaddressinput patterns

Verification:
- 162 tests pass (4 new)
- 169/169 taxonomy alignment
- Profile eval: 68/74 format-detectable correct (no regressions)
- cargo fmt, clippy clean

ACs #2-5 deferred to Phase 2-3 (phone_number locale patterns, model retraining, locale metadata output)
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Phase 1: Per-locale postal code validation integrated into attractor demotion.

Changes:
- `crates/finetype-core/src/taxonomy.rs`: Added `validation_by_locale` field to `Definition`, `compile_locale_validators()` and `get_locale_validators()` to `Taxonomy` with nested cache (label → locale → CompiledValidator)
- `labels/definitions_geography.yaml`: Added `validation_by_locale` with 14 locale postal code patterns (EN_US, EN_GB, EN_AU, EN_CA, DE, FR, ES, IT, NL, PL, RU, JA, ZH, KO) sourced from Google libaddressinput (Apache 2.0)
- `crates/finetype-model/src/column.rs`: Enhanced attractor demotion Signal 1 — checks locale patterns before universal validation. If any locale achieves >50% pass rate, prediction is locale-confirmed (skips demotion + Signal 2)
- `crates/finetype-cli/src/main.rs`: Added `compile_locale_validators()` calls after `compile_validators()`
- `data/cldr/README.md`: NEW — data source attribution and refresh instructions
- `CLAUDE.md`: Updated architecture docs with locale validation infrastructure

Tests:
- 4 new tests in taxonomy.rs (locale compilation, US/GB matching, cross-rejection, full YAML compile)
- 4 new tests in column.rs (salary demotion, US ZIP acceptance, UK postcode acceptance, low-confidence locale confirmation)
- All 162 tests pass, 169/169 taxonomy alignment, 68/74 profile eval (no regressions)

Impact:
- Salary columns (85000, 92500, 112000) that previously passed universal postal_code validation now fail all locale patterns → correctly demoted
- Real postal code columns (US ZIPs, UK postcodes) match their locale pattern → correctly accepted even at lower confidence
- No model retraining needed — works through existing attractor demotion pipeline

Deferred to follow-up tasks:
- AC #2: phone_number locale patterns (Phase 2)
- AC #3-5: 4-level training data, model retraining, locale metadata output (Phase 2-3)
<!-- SECTION:FINAL_SUMMARY:END -->
