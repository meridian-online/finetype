---
id: NNFT-121
title: >-
  Locale-specific validation Phase 2: phone_number patterns, 4-level labels,
  model retraining
status: To Do
assignee: []
created_date: '2026-02-25 01:21'
labels:
  - accuracy
  - locale
  - model-training
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
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Follow-up to NNFT-118 Phase 1 (per-locale postal code validation).

Phase 1 delivered locale-aware postal code validation via `validation_by_locale` in taxonomy YAML and attractor demotion Signal 1. This task covers the remaining scope:

1. **Phone number locale patterns** — Extract per-locale validation patterns from libphonenumber's PhoneNumberMetadata.xml for at least 10 countries. Add `validation_by_locale` to `phone_number` in definitions_identity.yaml.
2. **4-level training data** — Generate training data with locale suffix labels (e.g., `geography.address.postal_code.EN_US`) via `generate_all_localized()` in generator.rs.
3. **Model retraining** — Retrain CharCNN on ~484 locale-specific classes. Evaluate accuracy on profile eval.
4. **Locale metadata output** — Update inference pipeline to collapse 4-level predictions to 3-level user labels with locale as metadata field.

Data sources: libphonenumber (phone), libaddressinput (postal, already done), Google Address Data Service (address fields).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Per-locale validation regex patterns extracted for phone_number (at least 10 countries)
- [ ] #2 4-level training data generated via generate_all_localized()
- [ ] #3 Model retrained on 4-level labels with acceptable accuracy (no regression on profile eval)
- [ ] #4 Inference pipeline returns 3-level user label with locale metadata
- [ ] #5 phone_number false positives reduced on profile eval
<!-- AC:END -->
