---
id: NNFT-118
title: 'Discovery: locale-aware type inference for postal codes and phone numbers'
status: To Do
assignee: []
created_date: '2026-02-24 08:35'
labels:
  - discovery
  - accuracy
  - locale
dependencies: []
references:
  - labels/definitions_geography.yaml
  - labels/definitions_identity.yaml
  - crates/finetype-core/src/validator.rs
  - crates/finetype-core/src/taxonomy.rs
documentation:
  - discovery/locale-aware-inference/BRIEF.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Spike to answer the open questions in `discovery/locale-aware-inference/BRIEF.md`.

**Core tension:** Locale-specific types (postal_code, phone_number, names) try to be both general (all worldwide formats) and precise (discriminating from generic data). This is why postal_code is such a potent attractor — its validation can't reject salary integers or ticket numbers because it must also accept US ZIP, UK postcodes, Japanese postal codes, etc.

**The brief proposes locale-aware validation** where each type has per-locale regex patterns alongside the universal fallback. The spike should determine whether this is feasible, what the right architecture looks like, and where locale information comes from.

**Time budget:** 4 hours

**Research questions:**
1. Can locale be reliably inferred from the data itself? (e.g., postal code format fingerprinting, character set detection for names)
2. What schema format best represents per-locale validation? (inline YAML, separate files, JSON Schema if/then?)
3. How does locale flow through the inference pipeline? (column property? dataset context? external hint?)
4. Which types benefit most? (postal_code and phone_number are clear top two — what else?)
5. What does CLDR / libphonenumber offer as a data source for locale-specific patterns?
6. How does this interact with NNFT-116 (JSON Schema migration) and NNFT-117 (numeric range validation)?
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Discovery brief open questions answered with data and recommendations
- [ ] #2 Feasibility assessment: can locale be inferred from column data alone?
- [ ] #3 Proposed schema format for per-locale validation with concrete examples for postal_code and phone_number
- [ ] #4 Architecture sketch: how locale context flows through the inference pipeline
- [ ] #5 Identified data sources for locale-specific patterns (CLDR, libphonenumber, manual)
- [ ] #6 Written finding in discovery/locale-aware-inference/ with evidence, not just conclusions
<!-- AC:END -->
