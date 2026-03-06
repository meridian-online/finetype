---
id: NNFT-217
title: JSON profiling smoke tests and edge cases
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:15'
updated_date: '2026-03-06 03:51'
labels:
  - testing
  - json
milestone: m-9
dependencies:
  - NNFT-216
references:
  - tests/smoke.sh
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
End-to-end smoke tests for JSON profiling functionality. Tests cover nested objects, arrays of objects, mixed types, empty arrays, deeply nested structures, schema evolution across NDJSON lines, and top-level scalars (error case). Depends on NNFT-216 (CLI wiring).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Smoke test files created in tests/ covering JSON and NDJSON inputs
- [x] #2 Test: nested objects with dot notation paths (a.b.c)
- [x] #3 Test: arrays of objects with bracket notation (users[].email)
- [x] #4 Test: mixed types in arrays typed as VARCHAR
- [x] #5 Test: empty arrays handled gracefully
- [x] #6 Test: deeply nested structures (10+ levels)
- [x] #7 Test: schema evolution across NDJSON lines (missing fields → None)
- [x] #8 Test: top-level scalars produce appropriate error message
- [x] #9 All tests verify both plain and JSON output formats produce expected results
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Create test fixture files covering all edge cases
2. Write tests/smoke_json.sh using existing helpers framework
3. Test: nested objects, arrays, mixed types, empty arrays
4. Test: deeply nested (10+ levels), schema evolution, scalars
5. Test: both plain and JSON output formats
6. Run smoke tests, verify all pass
7. Run full test suite for regression check
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added comprehensive JSON profiling smoke tests (34 assertions, all passing).

## What Changed
- New test script: tests/smoke_json.sh
- 5 new test fixture files: nested_objects.json, mixed_array.json, empty_arrays.json, deeply_nested.json, schema_evolution.ndjson

## Test Coverage (10 sections, 34 assertions)
1. JSON/NDJSON auto-detection by extension
2. Nested objects with dot notation paths (user.contact.email)
3. Arrays of objects with bracket notation
4. Mixed types in arrays (tags[])
5. Empty arrays handled gracefully (no false paths)
6. Deeply nested structures (12 levels)
7. Schema evolution across NDJSON lines (missing fields)
8. Top-level scalars + malformed JSON error messages
9. All output formats (plain, json, csv) with JSON input
10. CSV regression check (existing profiling unaffected)

## Tests
- smoke_json.sh: 34/34 passing
- cargo test --lib: 258/258 passing
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
