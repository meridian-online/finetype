---
id: NNFT-211
title: Add format_string_alt to more datetime types
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
labels:
  - taxonomy
  - actionability
milestone: m-7
dependencies:
  - NNFT-203
references:
  - labels/definitions_datetime.yaml
  - crates/finetype-eval/src/bin/eval_actionability.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Survey actionability failures and add alt format strings to 3+ impactful datetime types (e.g., timestamps with/without seconds, dates with single-digit components). Depends on NNFT-203 which adds the field to the Definition struct.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 3+ datetime types gain format_string_alt values
- [ ] #2 Eval tests both primary and alt format strings
- [ ] #3 No actionability regression
- [ ] #4 Tests pass
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
