---
id: NNFT-075
title: >-
  Restructure boolean taxonomy from technology.development to
  representation.boolean
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-16 03:22'
updated_date: '2026-02-16 05:05'
labels:
  - taxonomy
  - model
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Move boolean classification from technology.development.boolean to the representation domain with format-specific subtypes. Booleans are a data representation concept, not a technology/development one. Splitting by string format enables better casting and normalization.

Current: technology.development.boolean (single catch-all)

Proposed:
- representation.boolean.binary — 0/1 values
- representation.boolean.initials — T/F, Y/N (single character)
- representation.boolean.terms — True/False, Yes/No, On/Off, Enabled/Disabled

This is a breaking taxonomy change requiring model retraining.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Remove technology.development.boolean from taxonomy
- [x] #2 Add representation.boolean.binary with generator for 0/1 values
- [x] #3 Add representation.boolean.initials with generator for T/F, Y/N variants
- [x] #4 Add representation.boolean.terms with generator for True/False, Yes/No, On/Off variants
- [x] #5 Generators produce case variants (TRUE/true/True, T/t, etc.)
- [x] #6 Column classifier rules updated for boolean subtypes
- [x] #7 finetype_cast normalization handles all three boolean formats
- [x] #8 DuckDB type mapping: all three map to BOOLEAN
- [x] #9 Model retrained with new labels
- [x] #10 Existing tests updated for new labels
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Phase 1 — Taxonomy & generators (no model change yet):
1. Update labels/definitions_representation.yaml: replace representation.logical.boolean with three subtypes (binary, initials, terms)
2. Remove technology.development.boolean from labels/definitions_technology.yaml
3. Update generator.rs: replace both boolean generators with three format-specific generators producing case variants
4. Run `finetype validate` to confirm taxonomy-generator alignment

Phase 2 — Downstream code:
5. Update BOOLEAN_LABELS in column.rs to reference new labels
6. Update header hints in column.rs (survived/alive etc.) to use representation.boolean.binary
7. Update normalize.rs: route all three boolean labels to normalize_boolean()
8. Update type_mapping.rs: map all three to BOOLEAN
9. Check unpack.rs for any boolean references
10. Update tests across all modified files

Phase 3 — Model retraining:
11. Generate training data with new labels (500 samples/label)
12. Train CharCNN v6 with updated taxonomy
13. Update models/default symlink
14. Run full validation suite
15. Run Titanic profile to verify no regressions
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Restructured boolean taxonomy from single `technology.development.boolean` to three format-specific subtypes under `representation.boolean.*`:

- `representation.boolean.binary` — 0/1 values
- `representation.boolean.initials` — T/F, Y/N (single char, any case)
- `representation.boolean.terms` — true/false, yes/no, on/off, enabled/disabled, active/inactive (any case)

Changes:
- **Taxonomy**: Replaced `technology.development.boolean` in definitions_technology.yaml with three new subtypes in definitions_representation.yaml under new `boolean` category
- **Generators**: Replaced old boolean generators with three format-specific generators producing full case variants (generator.rs)
- **Column classifier**: Updated BOOLEAN_LABELS constant with new labels plus legacy fallbacks; updated header hints for survived/alive columns to use `representation.boolean.binary` (column.rs)
- **Normalization**: Added routing for `("representation", "boolean")` to `normalize_boolean()` with 3 new tests (normalize.rs)
- **Type mapping**: All three subtypes map to BOOLEAN; kept legacy mapping with comment (type_mapping.rs)
- **JSON unpacking**: Changed JSON boolean literal annotation from `technology.development.boolean` to `representation.boolean.terms` (unpack.rs)
- **Model**: Retrained CharCNN v6 with 169 classes (was 168 in v5), 10 epochs, final accuracy 89.15%
- **Default model**: Updated symlink to char-cnn-v6

Validation:
- `finetype check`: 169/169 generators, 8450/8450 samples pass (100%)
- `cargo test --all`: 213 tests pass (73 + 109 + 31)
- `cargo fmt --check` and `cargo clippy`: clean
- Titanic profile: Survived column now correctly classified as `representation.boolean.binary` (was `technology.development.boolean`); all other columns unchanged
- Direct inference: true/false→terms, 0/1→binary, T/F/Y/N→initials — all correct
<!-- SECTION:FINAL_SUMMARY:END -->
