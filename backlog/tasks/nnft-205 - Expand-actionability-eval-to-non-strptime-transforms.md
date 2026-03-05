---
id: NNFT-205
title: Expand actionability eval to non-strptime transforms
status: To Do
assignee: []
created_date: '2026-03-04 20:14'
labels:
  - eval
  - actionability
milestone: m-7
dependencies: []
references:
  - crates/finetype-eval/src/bin/eval_actionability.rs
  - eval/eval_output/report.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Actionability eval only tests 33 types with format_string (strptime-based). 23 "Tier B" types (epochs, currency, JSON, numeric) have transforms but are untested. Extend eval to execute transform SQL via DuckDB to measure full transform coverage.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Eval tests transform-based types by executing DuckDB SQL
- [ ] #2 Report separates strptime vs transform results
- [ ] #3 Epoch + currency types included in eval
- [ ] #4 Single overall actionability metric still reported
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
