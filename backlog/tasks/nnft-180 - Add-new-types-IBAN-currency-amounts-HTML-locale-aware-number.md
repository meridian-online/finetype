---
id: NNFT-180
title: 'Add new types: IBAN, currency amounts, HTML, locale-aware number'
status: To Do
assignee: []
created_date: '2026-03-02 05:50'
labels:
  - taxonomy
  - v0.5.1
dependencies:
  - NNFT-179
references:
  - discovery/taxonomy-revision/BRIEF.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 4 of taxonomy revision (v0.5.1): add genuinely missing types identified by research.

New types to add:
1. **finance.banking.iban** — ISO 13616 International Bank Account Number. Up to 34 alphanumeric chars with country prefix + check digits. Strong detection signal via mod-97 check digit algorithm.
2. **finance.currency.amount_us** — Currency with US formatting ($1,234.56). Transform: strip symbol + commas → DECIMAL(18,2).
3. **finance.currency.amount_eu** — Currency with European formatting (€1.234,56). Transform: swap separators → DECIMAL(18,2).
4. **container.object.html** — HTML content (distinct from XML — allows unclosed tags, unquoted attributes). Transform: regexp_replace for tag stripping, DuckDB webbed extension for richer ops.
5. **representation.numeric.decimal_number_eu** — European decimal format (1.234,56 with period for thousands, comma for decimal). Transform: REPLACE chain → DOUBLE.

Optional (if time permits):
6. **finance.currency.amount_accounting** — Accounting format with parentheses for negatives: $(1,234.56).

Each type needs: YAML definition with validation + format_string + transform + samples, generator implementation, training data generation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 finance.banking.iban added with mod-97 validation
- [ ] #2 finance.currency.amount_us added with symbol-aware transform
- [ ] #3 finance.currency.amount_eu added with European separator transform
- [ ] #4 container.object.html added with tag detection and strip transform
- [ ] #5 representation.numeric.decimal_number_eu added with European decimal transform
- [ ] #6 Each new type has generator producing valid synthetic samples
- [ ] #7 cargo run -- check passes for all new types
- [ ] #8 cargo test passes
- [ ] #9 Training data generated for new types
- [ ] #10 Model retrained including new types
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
