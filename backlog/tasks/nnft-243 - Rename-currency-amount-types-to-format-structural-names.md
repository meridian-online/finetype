---
id: NNFT-243
title: Rename currency amount types to format-structural names
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 03:53'
updated_date: '2026-03-07 04:21'
labels:
  - taxonomy
  - naming
dependencies: []
references:
  - labels/definitions_finance.yaml
  - crates/finetype-model/src/column.rs
  - crates/finetype-model/src/label_category_map.rs
  - crates/finetype-core/src/generators/
  - eval/schema_mapping.yaml
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rename 7 currency amount types from locale/country-based names to format-structural names, consistent with the `decimal_number` / `decimal_number_comma` naming convention established in NNFT-233/234.

Rename mapping:
- `amount_us` → `amount`
- `amount_eu` → `amount_comma`
- `amount_accounting_us` → `amount_accounting`
- `amount_eu_suffix` → `amount_comma_suffix`
- `amount_space_sep` → `amount_space`
- `amount_indian` → `amount_lakh`
- `amount_ch` → `amount_apostrophe`

6 types unchanged (already format-structural): `amount_nodecimal`, `amount_code_prefix`, `amount_minor_int`, `amount_crypto`, `amount_multisym`, `amount_neg_trailing`.

Old names preserved in aliases arrays for backward compatibility.

Taxonomy-only change — no model retrain. Retrain deferred to a later batch with other planned taxonomy changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 amount_us → amount renamed in definitions_finance.yaml
- [x] #2 amount_eu → amount_comma renamed in definitions_finance.yaml
- [x] #3 amount_accounting_us → amount_accounting renamed
- [x] #4 amount_eu_suffix → amount_comma_suffix renamed
- [x] #5 amount_space_sep → amount_space renamed
- [x] #6 amount_indian → amount_lakh renamed
- [x] #7 amount_ch → amount_apostrophe renamed
- [x] #8 Old type names added to aliases array for each renamed type
- [x] #9 All references updated in codebase (column.rs, label_category_map.rs, header hints, generators, tests)
- [x] #10 `cargo run -- check` passes with correct type count
- [x] #11 `cargo test` and `make ci` pass
- [x] #12 eval/schema_mapping.yaml updated if any renamed types appear there
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## NNFT-243 Implementation Plan

### 1. Rename type keys in definitions_finance.yaml (7 renames)
- `finance.currency.amount_us` → `finance.currency.amount`
- `finance.currency.amount_eu` → `finance.currency.amount_comma`
- `finance.currency.amount_accounting_us` → `finance.currency.amount_accounting`
- `finance.currency.amount_eu_suffix` → `finance.currency.amount_comma_suffix`
- `finance.currency.amount_space_sep` → `finance.currency.amount_space`
- `finance.currency.amount_indian` → `finance.currency.amount_lakh`
- `finance.currency.amount_ch` → `finance.currency.amount_apostrophe`
- Add old name to aliases array for each

### 2. Update generators in generator.rs
- Rename match arms for all 7 types

### 3. Update label_category_map.rs
- Rename all 7 entries in NUMERIC_LABELS

### 4. Update column.rs references
- Search and replace any header hints or disambiguation references

### 5. Update DuckDB extension
- type_mapping.rs and normalize.rs if any of these types appear

### 6. Update training data
- data.rs Sense category mappings
- model2vec_prep.rs if any amount types referenced

### 7. Update eval mappings
- schema_mapping.csv and schema_mapping.yaml if any renamed types appear

### 8. Verify
- `cargo run -- check` (expect 207 types, same count)
- `cargo test`
- Grep for any remaining old names outside aliases
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Renamed 7 currency amount types from locale/country-based names to format-structural names, consistent with the `decimal_number` / `decimal_number_comma` naming convention.

**Renames (7):**
- `amount_us` → `amount` (period decimal is the default)
- `amount_eu` → `amount_comma`
- `amount_accounting_us` → `amount_accounting`
- `amount_eu_suffix` → `amount_comma_suffix`
- `amount_space_sep` → `amount_space`
- `amount_indian` → `amount_lakh`
- `amount_ch` → `amount_apostrophe`

**Unchanged (6):** `amount_nodecimal`, `amount_code_prefix`, `amount_minor_int`, `amount_crypto`, `amount_multisym`, `amount_neg_trailing` (already format-structural).

**Changes across codebase:**
- `labels/definitions_finance.yaml`: All 7 type keys renamed, old names added to aliases arrays, cross-reference notes updated
- `crates/finetype-core/src/generator.rs`: Match arms and test names updated
- `crates/finetype-model/src/label_category_map.rs`: All entries renamed and re-sorted alphabetically
- `crates/finetype-train/src/data.rs`: Sense category mappings updated
- No DuckDB extension or eval mapping changes needed (no hardcoded refs)

**Tests:** `cargo run -- check` (207/207, 100%), `cargo test` (405 passed). Grep confirms no stale old names outside aliases."
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
