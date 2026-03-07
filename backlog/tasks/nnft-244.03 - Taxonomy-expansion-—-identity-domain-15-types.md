---
id: NNFT-244.03
title: Taxonomy expansion — identity domain (+15 types)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 05:11'
updated_date: '2026-03-07 05:54'
labels:
  - taxonomy
  - expansion
  - identity
dependencies: []
references:
  - discovery/taxonomy-revision/EXPANSION.md
  - labels/definitions_identity.yaml
parent_task_id: NNFT-244
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add ~12 new identity types from EXPANSION.md Tiers 1-4:

**New categories:** medical (icd10, loinc, cpt, hcpcs), government (vin, eu_vat, ssn, ein, pan_india, abn), academic (orcid), commerce (upc, isrc)
**Existing categories:** person (email_display, phone_e164 — verify vs phone_number first)

PII-sensitive types: ssn, ein, pan_india need `pii: true` flag.
Disambiguation: CPT (5 digits) overlaps postal codes — header hints required.
Dedup check: phone_e164 vs phone_number.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 YAML definitions added for all identity types with validation, format_string, transform, broad_type, tier
- [x] #2 Generators produce valid samples that pass validation for each new type
- [x] #3 `finetype check` passes with all new identity types
- [x] #4 `finetype schema` exports valid JSON Schema for each new type
- [x] #5 PII types tagged with `pii: true` (ssn, ein, pan_india, eu_vat)
- [x] #6 Dedup check completed: phone_e164 vs phone_number — decision documented
- [x] #7 Locale-specific types have correct designation and validation_by_locale where applicable
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 15 new identity types across 5 new categories (medical, government, academic, commerce) and 1 existing category (person):

**New types:**
- `identity.medical.icd10` — ICD-10 diagnosis codes
- `identity.medical.loinc` — LOINC lab test codes
- `identity.medical.cpt` — CPT procedure codes
- `identity.medical.hcpcs` — HCPCS healthcare codes
- `identity.government.vin` — Vehicle Identification Numbers
- `identity.government.eu_vat` — EU VAT registration numbers (pii: true)
- `identity.government.ssn` — US Social Security Numbers (pii: true)
- `identity.government.ein` — US Employer Identification Numbers (pii: true)
- `identity.government.pan_india` — Indian PAN card numbers (pii: true)
- `identity.government.abn` — Australian Business Numbers
- `identity.academic.orcid` — ORCID researcher identifiers
- `identity.person.email_display` — Display-format email ("Name <email>")
- `identity.person.phone_e164` — E.164 international phone format
- `identity.commerce.upc` — Universal Product Codes (UPC-A)
- `identity.commerce.isrc` — International Standard Recording Codes

**PII tagged:** ssn, ein, pan_india, eu_vat (new) + existing types (email, phone_number, full_name, first_name, last_name, password).
**Dedup:** phone_e164 kept as distinct from phone_number — strict +CC format vs flexible locale patterns.

Commit: 4790e78
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
