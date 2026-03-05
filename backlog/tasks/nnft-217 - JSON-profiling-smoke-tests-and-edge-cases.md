---
id: NNFT-217
title: JSON profiling smoke tests and edge cases
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
updated_date: '2026-03-04 20:16'
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
End-to-end smoke tests for JSON profiling: depth limit (default 5), array sampling (default 100), heterogeneous arrays, deeply nested objects. Depends on NNFT-215 (CLI wiring).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Smoke test in tests/ directory
- [ ] #2 `--json-depth N` flag limits flattening depth
- [ ] #3 Array sampling defaults to 100 elements
- [ ] #4 Heterogeneous arrays typed as majority type
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
