---
id: NNFT-204
title: Fill transform stubs for julian date and rfc_2822_ordinal
status: To Do
assignee: []
created_date: '2026-03-04 20:14'
labels:
  - taxonomy
  - generators
milestone: m-7
dependencies: []
references:
  - labels/definitions_datetime.yaml
  - crates/finetype-core/src/generator.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Two datetime types have null transform AND null format_string — they're dead-end stubs. Add working DuckDB transforms and generators so these types are fully actionable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 julian_date has working DuckDB transform
- [ ] #2 rfc_2822_ordinal has working transform or documented limitation
- [ ] #3 `finetype check` passes
- [ ] #4 Generators produce valid samples for both types
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
