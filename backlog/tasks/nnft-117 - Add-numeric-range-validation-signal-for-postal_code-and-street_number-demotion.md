---
id: NNFT-117
title: Add numeric range validation signal for postal_code and street_number demotion
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-24 08:33'
updated_date: '2026-02-24 11:25'
labels:
  - accuracy
  - disambiguation
  - validation
dependencies:
  - NNFT-115
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extend the attractor demotion system (Rule 14, NNFT-115) with a numeric range signal that uses JSON Schema `maximum`/`minimum` keywords to catch implausible values.

**Problem:** postal_code has no regex pattern (only `minLength: 3, maxLength: 10`), so Signal 1 (validation failure) can never fire for it. Values like salary integers (41137–245486) and large ticket numbers (>99999) survive because they pass length checks and confidence exceeds the 0.85 threshold.

**Approach:** This is where JSON Schema validation shines. Add `minimum`/`maximum` constraints to postal_code's validation schema (e.g., `maximum: 99999` for numeric-only values, covering the practical ceiling of worldwide postal codes). Then ensure the validation pipeline checks these numeric constraints — currently `validate_value()` only checks `pattern`, `minLength`, `maxLength`, and `enum`.

This pairs naturally with NNFT-116 (JSON Schema validator migration) — a proper JSON Schema validator would handle `minimum`/`maximum` natively. If implemented before NNFT-116, extend the bespoke validator to check numeric range; if after, the JSON Schema validator handles it automatically.

**Driving examples:**
- `people_directory.salary`: values 41137–245486, predicted postal_code@0.91. Many values >99999 — implausible postal codes.
- `titanic.Ticket`: digit-only values range 1000–3101295, many >99999. (26% also contain letters/slashes that would fail a stricter pattern.)
- `sports_events.attendance`: values like 15000–85000, predicted postal_code@0.97.
- `network_logs.payload_size_bytes`: values 200–98000, predicted postal_code@0.93.

**Scope:** Add numeric range constraints to postal_code and street_number validation schemas. Extend validate_value() to check minimum/maximum if present. The attractor demotion Signal 1 then catches these automatically.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 postal_code validation schema includes maximum constraint (e.g., 99999) for numeric values
- [x] #2 street_number validation schema includes maximum constraint for numeric values
- [x] #3 validate_value() checks minimum/maximum constraints when present on string types parsed as numbers
- [x] #4 people_directory.salary no longer classified as postal_code on profile eval
- [x] #5 No regressions on actual postal code columns (geography_data.postal_code, ecommerce_orders.shipping_postal_code)
- [x] #6 Unit tests for numeric range validation on postal_code and street_number
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add maximum: 99999 to postal_code validation in definitions_geography.yaml
2. Add maximum: 99999 to street_number validation (reasonable ceiling for building numbers)
3. Run cargo test to verify schemas compile
4. Run eval to verify 3 FPs fixed (salary, Ticket, payload_size_bytes) with 0 regressions
5. Add unit tests for numeric range validation on postal_code
6. Run finetype check to verify generator alignment
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added maximum: 99999 numeric range constraint to postal_code and street_number validation schemas.

Changes:
- labels/definitions_geography.yaml: Added maximum: 99999 to postal_code validation (was only minLength: 3, maxLength: 10)
- labels/definitions_geography.yaml: Added maximum: 99999 to street_number validation (already had pattern constraint)
- crates/finetype-core/src/validator.rs: 2 new tests (postal_code_maximum_rejects_salary_range, street_number_maximum_rejects_large_values)

Results:
- 3 false positives fixed: salary→decimal_number, Ticket→alphanumeric_id, payload_size_bytes→alphanumeric_id
- Domain accuracy improved: titanic 91.7%→100%, people_directory 72.7%→81.8%, network_logs 16.7%→33.3%
- 0 regressions on real postal code columns (Indian 6-digit codes: only 9.5% exceed 99999, well under 50% demotion threshold)
- Format-detectable headline unchanged at 68/74 (fixed columns have partial/semantic_only GT labels)
- 249 tests pass (91 core + 158 model), no code changes needed — NNFT-116 CompiledValidator already handles minimum/maximum

Remaining postal_code FP: sports_events.attendance (max 79957, all values < 99999). Requires locale-specific types (NNFT-118) or additional signals.
<!-- SECTION:FINAL_SUMMARY:END -->
