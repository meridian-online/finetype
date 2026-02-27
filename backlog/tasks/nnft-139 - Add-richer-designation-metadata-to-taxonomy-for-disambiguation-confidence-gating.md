---
id: NNFT-139
title: >-
  Add richer designation metadata to taxonomy for disambiguation confidence
  gating
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-26 00:28'
updated_date: '2026-02-26 01:15'
labels:
  - taxonomy
  - disambiguation
  - accuracy
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Borrow the old finetype prototype's richer designation system (broad_words, broad_characters, broad_numbers, broad_object) and add it back to the current taxonomy YAML definitions. Use the designation in column disambiguation to gate confidence — broad_words types should automatically defer to header hints and receive a lower effective confidence since character patterns alone cannot reliably distinguish them.

The old repo (hughcameron/finetype) had designations: universal, locale_specific, broad_words, broad_characters, broad_numbers, broad_object, duplicate, system_internal. The current repo simplified to just universal/locale_specific. Restoring the richer set codifies institutional knowledge about which types the CharCNN can and can't distinguish.

Reference: decision-002 analysis of old repo design choices.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Taxonomy YAML definitions include a designation field with values beyond universal/locale_specific (at minimum: broad_words, broad_characters, broad_numbers)
- [x] #2 finetype-core parses the new designation values from YAML without error
- [x] #3 Column disambiguation in column.rs uses designation metadata to adjust confidence gating (broad_words predictions defer to header hints)
- [x] #4 All existing tests pass — no regressions
- [x] #5 cargo check/clippy/test clean
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### 1. Replace hardcoded `is_generic` with designation-aware check

In `crates/finetype-model/src/column.rs`, the `is_generic` check at line ~346 is a hardcoded match list. Replace it with a function that:

a. Keeps the existing hardcoded list as a fallback (for when taxonomy is not available)
b. When taxonomy IS available, looks up the winning type's designation:
   - `BroadWords` → is_generic = true (gender, occupation, degree, etc.)
   - `BroadCharacters` → is_generic = true (password, etc.)
   - `BroadNumbers` → is_generic = true (increment, day_of_month, etc.)
   - `BroadObject` → is_generic = true
   - `Universal` / `LocaleSpecific` → use existing hardcoded list only
c. Attractor-demoted predictions remain generic (existing behaviour preserved)
d. Boolean types remain generic (existing behaviour preserved)

### 2. Thread taxonomy through to header hint logic

The taxonomy is already available on `ColumnClassifier` via `self.taxonomy`. The `classify_column_with_header` method needs to pass it to the `is_generic` determination. Currently the header hint logic is inline in `classify_column_with_header` — extract the is_generic check into a helper function that accepts `Option<&Taxonomy>`.

### 3. Add Designation import to column.rs

Import `finetype_core::Designation` alongside the existing `finetype_core::Taxonomy` import.

### 4. Tests

- Add test: broad_words type (e.g., gender) defers to header hint even at high confidence
- Add test: universal type (e.g., email) resists header hint override at high confidence
- Verify all existing tests pass unchanged
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
All designation values (broad_words, broad_characters, broad_numbers) already existed in both the Rust enum and YAML definitions — AC #1 and #2 were pre-satisfied.

Implementation: extracted is_generic_prediction() helper function that consults taxonomy designation before falling back to hardcoded list. Added HARDCODED_GENERIC_LABELS constant. Import Designation from finetype_core.

Key behaviour change: with taxonomy present, locale_specific types like phone_number are NO LONGER generic (they were in the hardcoded list). This is correct — the designation says the model CAN classify them, and the hardcoded list was a workaround.

Tests: 7 new tests covering all designation variants, attractor demotion, boolean, and fallback paths. 191 total pass.

Fixed regression: is_generic_prediction() Signal 3 (designation lookup) was short-circuiting Signal 4 (hardcoded list) when taxonomy was present. Types like phone_number (locale_specific) and first_name (locale_specific) stopped being generic, breaking header hint overrides. Fix: reordered signals — hardcoded list (Signal 3) now runs BEFORE taxonomy designation (Signal 4), making designation additive. Profile eval restored to 70/74. All 197 tests pass, clippy clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Designation-aware confidence gating for column disambiguation (NNFT-139)

Added `is_generic_prediction()` function to `column.rs` that determines whether a prediction should defer to header hints. Uses four additive signals:

1. Attractor-demoted predictions → always generic
2. Boolean types → always generic
3. Hardcoded catch-all list (phone_number, first_name, iata_code, etc.) → always generic
4. Taxonomy designation lookup (broad_words, broad_characters, broad_numbers, broad_object) → additionally generic

Key design choice: Signal 3 (hardcoded list) runs BEFORE Signal 4 (taxonomy designation), making designation **additive**. This prevents types like phone_number (locale_specific) from losing their generic status when taxonomy is present. Initial implementation had Signal 4 short-circuiting Signal 3, causing a 70/74 → 63/74 regression — fixed by reordering.

The designation expansion enables types like gender, occupation, nationality (broad_words) to automatically defer to header hints without needing to be hardcoded individually.

Files changed:
- `crates/finetype-model/src/column.rs` — new `HARDCODED_GENERIC_LABELS` constant, `is_generic_prediction()` function, replaced inline match block

Tests: 197 pass (8 new), clippy clean, profile eval 70/74 (no regression).
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
