---
id: NNFT-118
title: 'Locale-specific type labels for postal codes, phone numbers, and addresses'
status: To Do
assignee: []
created_date: '2026-02-24 08:35'
updated_date: '2026-02-24 11:26'
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
- [ ] #1 Per-locale validation regex patterns extracted for postal_code (at least 10 countries)
- [ ] #2 Per-locale validation regex patterns extracted for phone_number (at least 10 countries)
- [ ] #3 4-level training data generated via generate_all_localized()
- [ ] #4 Model retrained on 4-level labels with acceptable accuracy
- [ ] #5 Inference pipeline returns 3-level user label with locale metadata
- [ ] #6 postal_code false positives reduced on profile eval
- [ ] #7 No regressions on format-detectable accuracy (68/74 baseline)
- [ ] #8 Data source attribution documented (libphonenumber, libaddressinput)
<!-- AC:END -->
