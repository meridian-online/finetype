---
id: NNFT-233
title: 'Taxonomy cleanup: remove 7 low-precision types, recategorize color types'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-06 08:30'
updated_date: '2026-03-06 08:56'
labels:
  - taxonomy
  - cleanup
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Remove 7 types that violate the Precision Principle (indistinguishable from generic types, no validation patterns, or enum-only with no structural signal). Move color_hex and color_rgb from representation.text to representation.format.

Removals (7):
- finance.payment.credit_card_network — enum of ~6 values, no pattern, just categorical
- technology.development.os — enum of ~5 values, just categorical
- technology.development.programming_language — enum, just categorical  
- technology.development.software_license — enum, just categorical
- technology.development.stage — enum, just ordinal
- technology.code.pin — 4-8 digit number, indistinguishable from integer_number/numeric_code
- datetime.component.day_of_month — 1-31 integer, indistinguishable from integer_number

Recategorization (2):
- representation.text.color_hex → representation.format.color_hex
- representation.text.color_rgb → representation.format.color_rgb

Net: 216 → 209 types. Requires model retrain (CharCNN-v13).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 7 types removed from YAML definitions
- [x] #2 2 color types moved to representation.format category
- [x] #3 Generators updated (remove 7, update category for 2)
- [x] #4 LabelCategoryMap updated for 209 types
- [x] #5 cargo run -- check passes (209/209)
- [x] #6 cargo test passes (type count assertions updated)
- [x] #7 CLAUDE.md updated with new type counts
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Remove 7 types from YAML definitions (3 files: finance, technology, datetime)
2. Move color_hex/color_rgb from representation.text to representation.format in YAML
3. Remove 7 generators from generator.rs
4. Update 2 generator match arms for color types (new category path)
5. Update LabelCategoryMap for 209 types
6. Update type count assertions in tests
7. cargo fmt + clippy + test + check
8. Update CLAUDE.md type counts
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Removed 7 low-precision types that violate the Precision Principle and recategorized 2 color types. Net taxonomy: 216 → 209 types.

**Removals (7):**
- `datetime.component.day_of_month` — 1-31 integer, indistinguishable from integer_number
- `finance.payment.credit_card_network` — enum of ~6 values, no structural pattern
- `technology.development.os` — enum, just categorical
- `technology.development.programming_language` — enum, just categorical
- `technology.development.software_license` — enum, just categorical
- `technology.development.stage` — enum, just ordinal
- `technology.code.pin` — 4-8 digit number, indistinguishable from integer_number/numeric_code

**Recategorization (2):**
- `representation.text.color_hex` → `representation.format.color_hex`
- `representation.text.color_rgb` → `representation.format.color_rgb`

**Files changed:**
- `labels/definitions_datetime.yaml` — removed day_of_month (26 lines)
- `labels/definitions_finance.yaml` — removed credit_card_network (34 lines)
- `labels/definitions_technology.yaml` — removed 5 types (230 lines)
- `labels/definitions_representation.yaml` — moved color types to format category
- `crates/finetype-core/src/generator.rs` — removed 7 generators, updated 2 category paths
- `crates/finetype-model/src/label_category_map.rs` — updated arrays, sorted, fixed all count assertions (209 total, 84 temporal, 28 finance, 19 technology, 51 format primary, 24 text)
- `CLAUDE.md` — updated taxonomy counts and version

**Verification:**
- `cargo test`: 402 tests pass (144 core + 258 model)
- `cargo run -- check`: 209/209 types pass, 100% alignment
- `cargo clippy`: clean (no warnings)
- Smoke tests: 25/25 pass

**Follow-up:** CharCNN-v13 retrain needed on 209-type taxonomy.
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
