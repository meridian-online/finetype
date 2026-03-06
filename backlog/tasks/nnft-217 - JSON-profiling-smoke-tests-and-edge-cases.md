---
id: NNFT-217
title: JSON profiling smoke tests and edge cases
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
updated_date: '2026-03-06 00:01'
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
- [ ] #1 Smoke test files created in tests/ covering JSON and NDJSON inputs
- [ ] #2 Test: nested objects with dot notation paths (a.b.c)
- [ ] #3 Test: arrays of objects with bracket notation (users[].email)
- [ ] #4 Test: mixed types in arrays typed as VARCHAR
- [ ] #5 Test: empty arrays handled gracefully
- [ ] #6 Test: deeply nested structures (10+ levels)
- [ ] #7 Test: schema evolution across NDJSON lines (missing fields → None)
- [ ] #8 Test: top-level scalars produce appropriate error message
- [ ] #9 All tests verify both plain and JSON output formats produce expected results
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
