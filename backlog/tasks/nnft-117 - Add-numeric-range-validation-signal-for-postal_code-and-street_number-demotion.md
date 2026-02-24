---
id: NNFT-117
title: Add numeric range validation signal for postal_code and street_number demotion
status: To Do
assignee: []
created_date: '2026-02-24 08:33'
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
- [ ] #1 postal_code validation schema includes maximum constraint (e.g., 99999) for numeric values
- [ ] #2 street_number validation schema includes maximum constraint for numeric values
- [ ] #3 validate_value() checks minimum/maximum constraints when present on string types parsed as numbers
- [ ] #4 people_directory.salary no longer classified as postal_code on profile eval
- [ ] #5 No regressions on actual postal code columns (geography_data.postal_code, ecommerce_orders.shipping_postal_code)
- [ ] #6 Unit tests for numeric range validation on postal_code and street_number
<!-- AC:END -->
