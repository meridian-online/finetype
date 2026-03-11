---
id: NNFT-272
title: >-
  Add numeric_code demotion rule (F5) — demote to integer_number when no leading
  zeros
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-10 23:50'
updated_date: '2026-03-11 00:36'
labels:
  - pipeline
  - disambiguation
dependencies: []
references:
  - crates/finetype-model/src/column.rs
  - labels/definitions_representation.yaml
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When `numeric_code` wins the column vote but no sampled values have leading zeros, demote to `integer_number` (BIGINT).

`numeric_code` exists specifically to preserve leading zeros (ZIP codes, NAICS, FIPS, etc.). Without leading zeros, the values are plain integers and should be typed as `integer_number`.

Discovered via `sports_events.csv` where `duration_minutes` (values: 180, 90, 120, 60) was misclassified as `numeric_code` (VARCHAR) instead of `integer_number` (BIGINT).

This is a new feature-based disambiguation rule (F5), following the pattern of F1–F4 in the Sense→Sharpen pipeline.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 New disambiguation rule F5: if column winner is `numeric_code` and zero sampled values have leading zeros, override to `integer_number` (BIGINT)
- [x] #2 Fallback is always `integer_number` — no need to check for decimals since numeric_code is integer-only by definition
- [x] #3 Rule fires after masked vote aggregation, alongside existing F1–F4 rules
- [x] #4 Existing F1 rule (leading-zero promotion TO numeric_code) remains unchanged and takes precedence
- [x] #5 Unit test covering: numeric_code winner with leading zeros (no demotion), numeric_code winner without leading zeros (demoted to integer_number)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add Rule F5 to `feature_disambiguate()` in `column.rs` (after F4, ~line 1876)
   - Check: `result.label == \"representation.identifier.numeric_code\"`
   - Check: `column_features.mean[feature_idx::HAS_LEADING_ZERO] < threshold` (use 0.01 — essentially zero, accounting for float imprecision)
   - Override: `result.label = \"representation.numeric.integer_number\"`
   - Set broad_type via label (integer_number maps to BIGINT naturally downstream)
   - Set disambiguation_rule tag: `feature_no_leading_zero:{ratio}`

2. Add two unit tests following F4 test pattern:
   - `test_rule_f5_numeric_code_demoted_without_leading_zeros` — numeric_code winner, HAS_LEADING_ZERO=0.0 → demoted to integer_number
   - `test_rule_f5_numeric_code_kept_with_leading_zeros` — numeric_code winner, HAS_LEADING_ZERO=0.5 → no demotion

3. Run `cargo test -p finetype-model` to verify
4. Run `cargo run -- check` to confirm taxonomy alignment
5. Update CLAUDE.md F5 reference in pipeline docs
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- Added Rule F5 to `feature_disambiguate()` after F4 (line ~1878 in column.rs)
- Reuses existing `leading_zero_ratio` variable already computed for F1
- Threshold 0.01 rather than 0.0 to handle float imprecision
- Two unit tests added: demotion case + no-demotion case
- Full test suite: 302 passed, 0 failed
- Taxonomy check: 250/250 definitions, 12500/12500 samples passed
- CLAUDE.md updated: Features line (F1-F4 → F1-F5), pipeline docs (5b), decided items (#22)"
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added feature-based disambiguation Rule F5 (NNFT-272): when `numeric_code` wins the column vote but no sampled values have leading zeros, demote to `integer_number` (BIGINT).

`numeric_code` exists to preserve leading zeros (ZIP codes, NAICS, FIPS). Without leading zeros, values are plain integers. This is the inverse of F1 (which promotes postal_code → numeric_code when leading zeros are present).

Changes:
- `crates/finetype-model/src/column.rs`: Added Rule F5 in `feature_disambiguate()` — checks `HAS_LEADING_ZERO < 0.01` threshold
- Two unit tests: `test_rule_f5_numeric_code_demoted_without_leading_zeros`, `test_rule_f5_numeric_code_kept_with_leading_zeros`
- `CLAUDE.md`: Updated Features (F1-F4 → F1-F5), pipeline docs (5b), decided items (#22)

Tests: `cargo test -p finetype-model` — 302 passed, 0 failed. `cargo run -- check` — 250/250 definitions, 100%.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
