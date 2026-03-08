---
id: NNFT-252
title: 'Bug: ticket_price classified correctly but output as VARCHAR in load command'
status: To Do
assignee: []
created_date: '2026-03-07 23:56'
labels:
  - bug
  - cli
dependencies: []
references:
  - crates/finetype-cli/src/main.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When running `finetype load -f sports_events.csv | duckdb`, the ticket_price column is correctly classified as `decimal_number` with 100% confidence and DOUBLE broad_type, but appears as bare VARCHAR in the CTAS output. The `build_load_expr()` function likely skips the transform because the column is marked `is_generic`.

Investigate why `is_generic` is true for decimal_number and fix the load command to apply the DOUBLE cast.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ticket_price column in sports_events.csv outputs with DOUBLE cast in load command
- [ ] #2 Root cause of is_generic=true for decimal_number identified and documented
- [ ] #3 Fix does not break other correctly-generic columns (e.g., boolean, text)
- [ ] #4 Load command smoke test covers numeric type casting
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
