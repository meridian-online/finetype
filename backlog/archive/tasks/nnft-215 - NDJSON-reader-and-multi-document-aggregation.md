---
id: NNFT-215
title: NDJSON reader and multi-document aggregation
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
labels:
  - core
  - json
milestone: m-9
dependencies:
  - NNFT-209
references:
  - crates/finetype-core/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Read NDJSON line-by-line, flatten each document, merge into single FlatTable with union of all paths. Handle schema evolution (different fields across lines) and top-level arrays. Depends on NNFT-209 (JSON flattener).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `flatten_ndjson()` reads NDJSON line-by-line
- [ ] #2 Documents with different fields produce union columns with NULLs for missing values
- [ ] #3 Top-level array auto-detected and handled
- [ ] #4 Tests cover schema evolution + top-level arrays
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
