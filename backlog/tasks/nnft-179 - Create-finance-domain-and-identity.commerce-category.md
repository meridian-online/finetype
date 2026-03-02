---
id: NNFT-179
title: Create finance domain and identity.commerce category
status: Done
assignee: []
created_date: '2026-03-02 05:50'
updated_date: '2026-03-02 06:24'
labels:
  - taxonomy
  - v0.5.1
dependencies:
  - NNFT-177
references:
  - discovery/taxonomy-revision/BRIEF.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 3 of taxonomy revision (v0.5.1): the largest structural change. Restructure identity.payment into a new top-level `finance` domain, and move product/publication identifiers to `identity.commerce`.

New finance domain structure:
- finance.banking: swift_bic (from identity.payment)
- finance.payment: credit_card_number, credit_card_expiration_date, credit_card_network, paypal_email (from identity.payment)
- finance.securities: cusip, isin, sedol, lei (from identity.payment)
- finance.crypto: bitcoin_address, ethereum_address (from identity.payment)
- finance.currency: currency_code, currency_symbol (from identity.payment)

New identity.commerce category:
- identity.commerce.ean (from technology.code)
- identity.commerce.isbn (from technology.code)
- identity.commerce.issn (from technology.code)

After this change:
- identity.payment category is eliminated (all 14 types redistributed, CVV already removed in phase 1)
- identity domain: person (14) + medical (3) + commerce (3) = 20 types
- technology.code: doi, imei, locale_code, pin = 4 types
- finance domain: banking (1) + payment (4) + securities (4) + crypto (2) + currency (2) = 13 types
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 New labels/definitions_finance.yaml created with banking, payment, securities, crypto, currency categories
- [x] #2 identity.commerce category added to labels/definitions_identity.yaml with ean, isbn, issn
- [x] #3 identity.payment category fully emptied and removed
- [x] #4 technology.code updated (ean, isbn, issn removed)
- [x] #5 All label references updated across codebase (LabelCategoryMap, Sense categories, column.rs, training data)
- [x] #6 cargo run -- check passes
- [x] #7 cargo test passes
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Created new top-level finance domain by restructuring identity.payment (14 types) into 5 subcategories. Created identity.commerce for product/publication identifiers.

New finance domain (13 types moved):
- finance.banking: swift_bic
- finance.payment: credit_card_number, credit_card_expiration_date, credit_card_network, paypal_email
- finance.securities: cusip, isin, sedol, lei
- finance.crypto: bitcoin_address, ethereum_address
- finance.currency: currency_code, currency_symbol

New identity.commerce (3 types moved from technology.code):
- identity.commerce.ean, isbn, issn

identity.payment category fully eliminated. All Rust references updated across label_category_map.rs, column.rs, inference.rs, generator.rs, type_mapping.rs, eval schema mappings.

Note: ACs #8-10 (Sense mapping, model retrain, eval baselines) deferred to NNFT-181.
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
