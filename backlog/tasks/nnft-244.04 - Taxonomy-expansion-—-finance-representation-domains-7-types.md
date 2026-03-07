---
id: NNFT-244.04
title: Taxonomy expansion — finance + representation domains (+7 types)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 05:11'
updated_date: '2026-03-07 05:54'
labels:
  - taxonomy
  - expansion
  - finance
  - representation
dependencies: []
references:
  - discovery/taxonomy-revision/EXPANSION.md
  - labels/definitions_finance.yaml
  - labels/definitions_representation.yaml
parent_task_id: NNFT-244
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add ~7 new types from EXPANSION.md Tiers 1-4:

**Finance (+3):** figi (securities), aba_routing (banking), bsb (banking)
**Representation (+4):** cas_number (scientific), inchi (scientific), smiles (scientific), color_hsl (text)

Disambiguation: SMILES overlaps plain text — needs broad_characters designation, column-level only.
PII: aba_routing and bsb should get `pii: true` (banking details).
Locale-specific: bsb (EN_AU), aba_routing (EN_US).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 YAML definitions added for all finance + representation types
- [x] #2 Generators produce valid samples that pass validation for each new type
- [x] #3 `finetype check` passes with all new types
- [x] #4 `finetype schema` exports valid JSON Schema for each new type
- [x] #5 SMILES type has broad_characters designation
- [x] #6 Locale-specific types (bsb, aba_routing) have correct designation and validation_by_locale
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 7 new types across finance (3) and representation (4) domains:

**Finance:**
- `finance.securities.figi` — Financial Instrument Global Identifier
- `finance.banking.aba_routing` — ABA routing transit numbers (locale: EN_US)
- `finance.banking.bsb` — Australian BSB bank codes (locale: EN_AU)

**Representation:**
- `representation.scientific.cas_number` — CAS Registry Numbers (chemical substances)
- `representation.scientific.inchi` — InChI chemical structure identifiers
- `representation.scientific.smiles` — SMILES molecular notation (broad_characters designation)
- `representation.text.color_hsl` — HSL/HSLA color values

**Locale-specific:** aba_routing (EN_US), bsb (EN_AU) with validation_by_locale patterns.
**SMILES:** Designated broad_characters — column-level inference only, too permissive for value-level.

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
