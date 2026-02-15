---
id: NNFT-062
title: Fix port disambiguation false positive in column mode
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-15 05:12'
updated_date: '2026-02-15 07:14'
labels:
  - bugfix
  - disambiguation
dependencies: []
references:
  - crates/finetype-model/src/column.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `numeric_port_detection` disambiguation rule in `column.rs` fires incorrectly on columns like Titanic's Age (values 0-80). The bug: `has_common_ports` uses `.any()` — a single matching value (e.g., 22, 25, 53 which are both common ages AND common ports) is enough to trigger the rule.

Fix: require that a significant fraction (≥30%) of parsed values match the common port list, not just "any". Real port columns will have many common ports; age/count columns will coincidentally match just a few.

File: `crates/finetype-model/src/column.rs`, function `disambiguate_numeric()`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Port disambiguation requires ≥30% of values to match common port list (not just any)
- [x] #2 Titanic Age column no longer classified as technology.internet.port
- [x] #3 Existing port detection unit test still passes with real port data
- [x] #4 New unit test: column of ages (22, 25, 30, 35, 40, 45, 50, 53, 60, 70) does NOT trigger port detection
- [x] #5 All existing column.rs tests pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Read the `disambiguate_numeric()` function in column.rs (done — lines 334-502)
2. Change port detection from `.any()` to a fraction-based check (≥30% of values must match common ports)
3. Add unit test: column of ages (22, 25, 30, 35, 40, 45, 50, 53, 60, 70) does NOT trigger port detection
4. Verify existing port detection test still passes with real port data
5. Run full test suite
6. Verify Titanic Age column no longer classified as port (requires build + profile)
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed the port disambiguation false positive in `crates/finetype-model/src/column.rs`.

**Root cause**: The `has_common_ports` check used `.any()` — a single value matching the common port list (e.g., 22, 25, 53) was enough to trigger `numeric_port_detection`. The Titanic Age column (values 0-80) had several values that coincidentally matched common port numbers, causing it to be classified as `technology.internet.port` with 0.96 confidence.

**Fix**: Changed from `.any()` to a fraction-based check requiring ≥30% of parsed values to match the common port list. Real port columns (80, 443, 8080, etc.) easily exceed this threshold. Age/count columns with a few coincidental matches (22, 25, 53) fall well below it.

**Results**:
- Titanic Age column: `technology.internet.port` → `identity.person.age` (0.96 confidence)
- Existing port detection test: still passes (10/10 = 100% match rate)
- 2 new unit tests added for age column patterns
- All 165 tests pass (64 in finetype-model, including 24 column tests)
- All 25 smoke tests pass
<!-- SECTION:FINAL_SUMMARY:END -->
