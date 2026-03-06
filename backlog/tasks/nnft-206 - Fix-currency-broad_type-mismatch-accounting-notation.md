---
id: NNFT-206
title: Fix currency broad_type mismatch + accounting notation
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:14'
updated_date: '2026-03-06 04:32'
labels:
  - taxonomy
  - finance
milestone: m-7
dependencies: []
references:
  - labels/definitions_finance.yaml
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`amount_us`/`amount_eu` declare `broad_type: VARCHAR` but their transforms produce DECIMAL — a mismatch that would break schema-for DDL generation. Also add accounting notation support for parenthesized negatives like `($1,234.56)`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 broad_type corrected to match transform output type
- [x] #2 Validation accepts accounting notation e.g. ($1,234.56)
- [x] #3 Generator produces accounting-notation samples
- [x] #4 `finetype check` passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Change broad_type from VARCHAR to DECIMAL for amount_us and amount_eu
2. Update amount_us validation regex to also accept parenthesized negatives like ($1,234.56)
3. Update amount_us transform to handle parentheses → negative
4. Add accounting-notation samples to generator
5. Run cargo test + cargo run -- check

Note: amount_accounting already exists as a separate type. This task extends amount_us to also handle accounting notation as a variant.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed broad_type mismatch and added accounting notation support for US currency amounts.

Changes:
- `amount_us` and `amount_eu`: Changed `broad_type` from `VARCHAR` to `DECIMAL` to match transform output (DECIMAL(18,2))
- `amount_us` transform: Added CASE WHEN for parenthesized negatives — `($1,234.56)` → `-1234.56`
- `amount_us` validation: Extended regex with `^\\([$...]?[0-9,]+(\\.[0-9]{1,2})?\\)$` alternative
- `amount_us` generator: ~5% of samples now use accounting notation `($X,XXX.XX)`
- Added `($1,234.56)` and `($999.99)` to YAML samples

Tests: `cargo test` (258 passed), `cargo run -- check` (216/216, 10800/10800). Transform verified in DuckDB CLI for both standard and accounting inputs.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
