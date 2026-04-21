---
status: accepted
date-created: 2026-03-02
date-modified: 2026-03-11
---
# 0007. Taxonomy revision v0.5.1 — finance domain, identifier category, type additions/removals

## Context and Problem Statement

FineType v0.5.0 had 163 types across 6 domains after the Phase 0 taxonomy audit (NNFT-162). External research (two independent agents surveying Kaggle, government data, enterprise SaaS schemas, and analyst pain points) identified coverage gaps in finance and identity domains. The `identity.payment` category mixed securities, crypto, and payment instruments — 14 types in one category.

The question: what taxonomy changes deliver the highest analyst value while maintaining the Precision Principle (each type must be a meaningful transformation contract)?

## Considered Options

- **Option A — Minimal additions only.** Add 5-7 missing types (currency with symbol, European decimal, etc.) without restructuring. Lower risk, but leaves the `identity.payment` category bloated.
- **Option B — Finance domain + identifier category + targeted additions.** Create a new `finance` domain (extracting banking/commerce from `identity.payment`), add an `identifier` category for codes that aren't person/payment/medical, plus 5-7 new types. Higher impact restructuring.

## Decision Outcome

Chosen option: **Option B — Finance domain, identifier category, and targeted additions**, because:
1. The `identity.payment` category was too heterogeneous — IBAN and Bitcoin address serve fundamentally different transformation contracts
2. A dedicated `finance` domain with `banking` and `commerce` categories creates clear homes for existing and future financial types
3. The `identifier` category captures numeric codes (NAICS, FIPS, etc.) that are neither person IDs nor payment instruments

Net result: 163 → 164 types across 7 domains. The finance domain launched with banking (IBAN, SWIFT, ABA routing, BSB) and commerce (currency amount variants, FIGI) categories. The `identifier` category added `numeric_code` (VARCHAR preserving leading zeros).

### Consequences

- Good, because the taxonomy now has a natural home for future financial types (stock tickers, exchange codes, etc.)
- Good, because `numeric_code` solves the integer vs code ambiguity — codes with leading zeros stay VARCHAR
- Good, because splitting `identity.payment` into focused categories improves type discoverability
- Bad, because the restructuring requires CharCNN retraining on the new label space
- Bad, because 2 types were removed (http_status_code, port) — users relying on these predictions lose them
