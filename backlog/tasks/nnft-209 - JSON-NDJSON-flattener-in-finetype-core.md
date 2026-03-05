---
id: NNFT-209
title: JSON/NDJSON flattener in finetype-core
status: To Do
assignee: []
created_date: '2026-03-04 20:14'
labels:
  - core
  - json
milestone: m-9
dependencies: []
references:
  - crates/finetype-core/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
New `json_flatten` module in finetype-core. Converts JSON to tabular representation: field paths as column names, values as column vectors.

Examples:
- `{"a":{"b":1}}` → column `a.b`
- `[{"x":1},{"x":2}]` → column `x`

This is foundational for the entire JSON profiling milestone (m-9).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `flatten_json()` returns `FlatTable` struct
- [ ] #2 FlatTable has columns with path + values vectors
- [ ] #3 Nested objects produce dot-separated paths
- [ ] #4 Arrays produce `[]` notation in paths
- [ ] #5 Unit tests cover nesting, arrays, and null handling
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
