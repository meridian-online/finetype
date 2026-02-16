---
id: NNFT-076
title: Improve small-integer column disambiguation (Pclass/SibSp/Parch problem)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-16 03:22'
updated_date: '2026-02-16 03:53'
labels:
  - model
  - column-classifier
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Columns containing small integers (0-9) are frequently misclassified. Observed on Titanic dataset:
- Pclass {1,2,3} → datetime.component.day_of_month (should be ordinal/categorical)
- SibSp {0,1,2,3,4,5,8} → technology.development.boolean (should be integer count)
- Parch {0,1,2,3,4,5,6} → technology.development.boolean (should be integer count)

Small integers are genuinely ambiguous at the value level. Column-level signals (cardinality, header name, value distribution) are needed to disambiguate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Header hints added for common count/class columns (pclass, class, siblings, parents, children, count, qty, quantity)
- [x] #2 Column classifier distinguishes boolean (exactly 2 unique values from {0,1,true,false,...}) from integer counts (small integers with >2 unique values)
- [x] #3 Pclass-like columns ({1,2,3} with 'class' header) classified as ordinal or categorical, not day_of_month
- [x] #4 SibSp/Parch-like columns (small integer counts) not misclassified as boolean
- [x] #5 Titanic dataset profile shows improved classifications for Pclass, SibSp, Parch
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Centralize boolean label constants to avoid label mismatches
2. Fix disambiguate_boolean_override() to check actual model label
3. Fix disambiguate_categorical() and classify_column_with_header() generic type lists
4. Add header hints for class/count/survival/ticket/cabin/embarked/fare columns
5. Add small-integer ordinal disambiguation rule
6. Add comprehensive tests for all new behavior
7. Verify Titanic dataset profile improvement
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed critical label mismatch bug in column classifier and added new disambiguation rules for small-integer columns.

## Root Cause

The `disambiguate_boolean_override()` function checked for `representation.logical.boolean` and `technology.data.boolean` but the actual CharCNN v5 model outputs `technology.development.boolean`. This meant the boolean override rule **never fired**, causing SibSp/Parch to be misclassified as boolean when they have >2 unique integer values.

The same stale labels appeared in `disambiguate_categorical()` and `classify_column_with_header()` generic type checks.

## Changes

### Bug fixes (crates/finetype-model/src/column.rs)
- Centralized all boolean label variants into `BOOLEAN_LABELS` constant to prevent future mismatches
- Updated `disambiguate_boolean_override()` to use `BOOLEAN_LABELS` (now fires correctly)
- Updated `disambiguate_categorical()` generic_types to include `BOOLEAN_LABELS` and `day_of_month`
- Updated `classify_column_with_header()` is_generic check to use `BOOLEAN_LABELS` and include `day_of_month`

### New features
- Added `disambiguate_small_integer_ordinal()` rule: detects ordinal patterns (e.g., {1,2,3} ratings) when model predicts day_of_month for small repeated integer sets
- Added 30+ header hints: class/rank/grade/tier → ordinal, sibsp/parch/siblings/children/qty → integer_number, survived/alive/active → boolean, ticket/cabin/seat → alphanumeric_id, embarked/terminal → categorical, fare/fee → decimal_number
- Added keyword matching for compound header names (e.g., "passenger_class", "ticket_class")

### Tests
- 11 new unit tests covering:
  - Boolean override with actual model label `technology.development.boolean`
  - Small-integer ordinal detection (Pclass, ratings)
  - Ordinal skip for boolean {0,1} and large-range integers
  - Header hints for class, count, survival, ticket/cabin, embarked, fare columns

## Titanic Dataset Results

| Column | Before | After |
|--------|--------|-------|
| Pclass | day_of_month ❌ | integer_number ✅ |
| SibSp | boolean ❌ | integer_number ✅ |
| Parch | boolean ❌ | integer_number ✅ |

All other columns remain correctly classified. Full suite: 182 tests pass, fmt clean, clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->
