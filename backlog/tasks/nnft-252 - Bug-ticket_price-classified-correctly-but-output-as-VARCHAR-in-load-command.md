---
id: NNFT-252
title: 'Bug: ticket_price classified correctly but output as VARCHAR in load command'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 23:56'
updated_date: '2026-03-08 01:23'
labels:
  - bug
  - cli
dependencies: []
references:
  - crates/finetype-cli/src/main.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When running `finetype load -f sports_events.csv | duckdb`, the ticket_price column is correctly classified as `decimal_number` with 100% confidence and DOUBLE broad_type, but appears as bare VARCHAR in the CTAS output. The `build_load_expr()` function likely skips the transform because the column is marked `is_generic`.

Investigate why `is_generic` is true for decimal_number and fix the load command to apply the DOUBLE cast.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ticket_price column in sports_events.csv outputs with DOUBLE cast in load command
- [x] #2 Root cause of is_generic=true for decimal_number identified and documented
- [x] #3 Fix does not break other correctly-generic columns (e.g., boolean, text)
- [x] #4 Load command smoke test covers numeric type casting
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Root Cause

`decimal_number` and `integer_number` are in `HARDCODED_GENERIC_LABELS` (column.rs:120), so `is_generic=true`. This is correct for classification — these types should yield to header hints. But `build_load_expr()` (main.rs:2295) uses `is_generic` to skip the CAST, conflating classification uncertainty with cast safety.

## Fix

1. In `build_load_expr()`, remove `is_generic` from the skip condition — only check `duckdb_type == \"VARCHAR\"`
2. This means generic types with non-VARCHAR broad_types (BIGINT, DOUBLE) will get their casts applied
3. Verify: boolean (VARCHAR broad_type) still stays VARCHAR; text types still stay VARCHAR
4. Add test covering numeric type casting in load output
5. Run cargo test + check"
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Root cause: Two places in the load command conflated `is_generic` (classification uncertainty) with cast safety:
1. Line 2185: `final_type` was overridden to VARCHAR when `is_generic=true` — even for types like decimal_number (DOUBLE) and integer_number (BIGINT)
2. Line 2295: `build_load_expr()` skipped CAST when `is_generic || duckdb_type == \"VARCHAR\"`

`decimal_number` and `integer_number` are in HARDCODED_GENERIC_LABELS because they're catch-all types that should yield to header hints during classification. But their taxonomy broad_type (DOUBLE, BIGINT) is still meaningful for casting.

Fix: Removed is_generic from both the final_type override and build_load_expr. The broad_type from taxonomy now flows through directly. VARCHAR types naturally skip the cast. Removed is_generic field from LoadColumn struct (no longer needed in load path)."
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed `finetype load` outputting bare VARCHAR for generic types that have meaningful non-VARCHAR casts (decimal_number→DOUBLE, integer_number→BIGINT).

## Root Cause

`is_generic` conflated two concerns:
1. **Classification uncertainty** — generic types yield to header hints (correct, unchanged)
2. **Cast safety** — load command skipped CAST for generic types (incorrect)

`decimal_number` is in `HARDCODED_GENERIC_LABELS` for classification, but its taxonomy `broad_type` is DOUBLE with transform `CAST({col} AS DOUBLE)`. The load command was overriding this to VARCHAR.

## Changes

- `crates/finetype-cli/src/main.rs`:
  - Removed `is_generic` override of `final_type` — broad_type flows through from taxonomy
  - Removed `is_generic` parameter from `build_load_expr()` — only `duckdb_type == \"VARCHAR\"` skips cast
  - Removed `is_generic` field from `LoadColumn` struct (unused in load path)
  - Added 5 unit tests for `build_load_expr` covering VARCHAR, DOUBLE, BIGINT, BOOLEAN, and alias scenarios

## Before/After

```
-- Before: ticket_price bare VARCHAR
ticket_price,  -- representation.numeric.decimal_number

-- After: proper CAST
CAST(ticket_price AS DOUBLE) AS ticket_price,  -- representation.numeric.decimal_number
```

## Tests

- 5 new `test_build_load_expr_*` tests — all pass
- Full suite: 436 tests pass, 0 failures
- Manual verification on sports_events.csv confirms DOUBLE cast applied"
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
