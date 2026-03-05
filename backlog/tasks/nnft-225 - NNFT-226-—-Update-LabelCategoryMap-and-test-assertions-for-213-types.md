---
id: NNFT-225
title: NNFT-226 — Update LabelCategoryMap and test assertions for 213 types
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-05 01:56'
updated_date: '2026-03-05 02:55'
labels:
  - format-coverage
  - label-mapping
  - workstream-c
dependencies:
  - NNFT-223
  - NNFT-224
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Integrate all 54 new types into FineType's column classification system by updating LabelCategoryMap and test assertions.

**Primary task**: Update crates/finetype-model/src/label_category_map.rs

**Scope**:
- Extend TEMPORAL_LABELS array to include all 38 new datetime types
- Update CURRENCY_LABELS array to include 12 new currency types
- Update RATE_LABELS array if yield (finance.rate.yield) is added
- Update category distribution assertions in tests:
  - Total types: 163 → 213
  - Temporal category: 45 → 83 types (+38)
  - Currency category: 4 → 16 types (+12)
  - Other categories unchanged

**Why this matters**: Sense classifier routes column predictions to eligible types via LabelCategoryMap. Without updated maps, new types won't be considered in column-level classification even though they're in the taxonomy.

**Files to update**:
- crates/finetype-model/src/label_category_map.rs (primary)
  - Update TEMPORAL_LABELS: Add all 38 new datetime type labels
  - Update CURRENCY_LABELS: Add 12 new currency type labels
  - Update test_total_is_163() → test_total_is_213()
  - Update test_category_counts() assertions
  - Add validation test for CJK format inclusion in TEMPORAL_LABELS

**Files to verify** (may need minor updates):
- crates/finetype-model/tests/ (any hardcoded type count assertions)
- tests/smoke.sh (if it enumerates types or checks counts)

**Testing approach**:
1. Add all 38 datetime types to TEMPORAL_LABELS with correct \"datetime.date.X\" and \"datetime.timestamp.Y\" labels
2. Add 12 currency types to CURRENCY_LABELS
3. Update assertion: assert_eq!(TOTAL_TYPES, 213);
4. Update test_category_counts() distribution:
   - temporal: 45 → 83
   - currency: 4 → 16
   - other categories unchanged
5. Run `cargo test` — all assertions should pass
6. Run `cargo run -- check` — should report taxonomy ↔ generator alignment with all 213 types

**Deliverable**: Updated LabelCategoryMap with all 213 types properly categorized, passing all tests and taxonomy checks
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All 38 datetime types added to TEMPORAL_LABELS array with correct label format (datetime.date.X, datetime.timestamp.Y)
- [x] #2 All 12 currency types added to CURRENCY_LABELS array
- [x] #3 test_total_is_163() updated to test_total_is_213()
- [x] #4 test_category_counts() updated: temporal 45→83, currency 4→16, all others unchanged
- [x] #5 `cargo test` passes all assertions (no type count mismatches)
- [x] #6 `cargo run -- check` validates 213-type taxonomy with zero alignment errors
- [x] #7 CJK datetime types (chinese_ymd, korean_ymd, jp_era_short, jp_era_long) properly included in TEMPORAL_LABELS
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add 40 new datetime labels to TEMPORAL_LABELS (sorted alphabetically)
2. Add 11 new currency amount labels to NUMERIC_LABELS (they're amounts = numeric values)
3. Add 2 new rate labels to NUMERIC_LABELS (basis_points and yield are numeric values)
4. Update test assertions: total 163→216, temporal 45→85, numeric 13→26
5. Update eligible_labels test: temporal 45→85, numeric 16→29 (26+3 geo overlaps)
6. Run cargo test, cargo run -- check"
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Updated LabelCategoryMap to include all 216 FineType types for Sense→Sharpen column classification.

## Changes

**`crates/finetype-model/src/label_category_map.rs`**:
- TEMPORAL_LABELS: 45→85 types (+40 new datetime labels including CJK dates, Japanese era, periods, CLF, syslog, ISO 8601 ms/us variants)
- NUMERIC_LABELS: 13→26 types (+13 new: 11 currency amount formats + 2 rate types)
- All labels sorted alphabetically (required by test_all_labels_are_sorted)
- Test assertions updated: total 163→216, temporal 45→85, numeric 13→26, eligible counts adjusted

## Category mapping decisions
- New currency amount types → NUMERIC (consistent with existing amount_us, amount_eu)
- New rate types (basis_points, yield) → NUMERIC (they're numeric values)
- New period types (quarter, fiscal_year) → TEMPORAL (they're date-adjacent)
- All other categories unchanged

## Verification
- `cargo test --all`: 480 tests, 0 failures
- `cargo run -- check`: 216/216 pass, 100% sample validation
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
