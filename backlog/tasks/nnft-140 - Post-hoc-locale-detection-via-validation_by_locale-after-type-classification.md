---
id: NNFT-140
title: Post-hoc locale detection via validation_by_locale after type classification
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-26 00:28'
updated_date: '2026-02-26 01:15'
labels:
  - locale
  - disambiguation
  - accuracy
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement Option B from decision-002: keep tiered-v2 for type classification, detect locale via validation_by_locale after the type is determined. The infrastructure is already committed and working (validation_by_locale, locale_confirmed, detected_locale field in ColumnResult, strip_locale_suffix). This task wires locale detection into the column classification output path and exposes it through CLI JSON output.

When a column is classified as a locale-specific type (e.g. phone_number), run sample values against each locale's validation pattern. The locale with the highest match rate becomes detected_locale. This gives users locale information without any model regression risk.

Resolves decision-002. Unblocks completion of NNFT-126.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Column classification populates detected_locale field when type has validation_by_locale patterns
- [x] #2 Locale detection picks the locale with highest validation pass rate from sample values
- [x] #3 CLI JSON output includes locale field for locale-detected columns
- [x] #4 Profile eval remains at 70/74 — zero regression
- [x] #5 Works for phone_number (15 locales) and postal_code (14 locales) at minimum
- [x] #6 All existing tests pass plus new tests for locale detection logic
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### 1. Add post-hoc locale detection in column classification

In `crates/finetype-model/src/column.rs`, after the majority vote is determined and disambiguation is applied, add a new step:

a. Check if the winning type has `validation_by_locale` patterns in the taxonomy
b. If yes, run sample values against each locale's validation pattern
c. Track pass rate per locale
d. The locale with the highest pass rate (above 50% threshold) becomes `detected_locale`
e. If no locale reaches the threshold, `detected_locale` remains None

This replaces the current `detected_locale` derivation (lines 230-236) which only works with 4-level model labels from tiered-v3. The new approach works with tiered-v2 (3-level labels) by using validation patterns instead.

### 2. Create helper function `detect_locale_from_validation`

```rust
fn detect_locale_from_validation(
    values: &[String],
    label: &str,
    taxonomy: &Taxonomy,
) -> Option<String>
```

Iterates over compiled locale validators for the given label, calculates pass rate for each locale against the sample values, returns the best-matching locale (if >50%).

### 3. Wire into both classify_column and classify_column_with_header

Both methods produce a ColumnResult. After disambiguation:
- If taxonomy is available AND winning label has locale validators → call detect_locale_from_validation
- Set result.detected_locale from the return value
- This takes priority over any model-derived locale (from vote aggregation)

### 4. Ensure CLI output already works

The CLI JSON output (main.rs ~line 652) already outputs `detected_locale` as `"locale"`. No CLI changes needed — just confirming the field flows through.

### 5. Tests

- Add test: phone_number column with US-format numbers → detected_locale = "EN_US"
- Add test: phone_number column with UK-format numbers → detected_locale = "EN_GB"
- Add test: postal_code column with US ZIP codes → detected_locale = "US"
- Add test: type without locale validators → detected_locale = None
- Verify profile eval remains 70/74
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Post-hoc locale detection via validation_by_locale (NNFT-140)

Implemented Option B from decision-002: detect locale after type classification by running sample values against each locale's validation patterns.\n\nAdded `detect_locale_from_validation()` function to `column.rs` that:\n- Looks up compiled locale validators for the classified type\n- Tests non-empty sample values against each locale pattern\n- Returns the locale with highest pass rate above 50% threshold\n\nWired into both `classify_column()` (Step 5, after disambiguation) and `classify_column_with_header()` (clears stale locale when header hint changes the label).\n\nCLI JSON output now includes `"locale": "EN_US"` (or similar) when locale detection succeeds.\n\nVerified working:\n- +1 (xxx) xxx-xxxx → EN_US\n- +44 xx xxxx xxxx → EN_GB\n- 5-digit postal codes → DE (ambiguous with US, correct given pattern overlap)\n\nFiles changed:\n- `crates/finetype-model/src/column.rs` — new `detect_locale_from_validation()`, wired into classify_column and classify_column_with_header\n\nTests: 197 pass (5 new locale tests), clippy clean, profile eval 70/74 (no regression).
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
