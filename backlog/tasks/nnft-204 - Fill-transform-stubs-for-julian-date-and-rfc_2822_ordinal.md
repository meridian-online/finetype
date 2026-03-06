---
id: NNFT-204
title: Fill transform stubs for julian date and rfc_2822_ordinal
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:14'
updated_date: '2026-03-06 04:28'
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
- [x] #1 julian_date has working DuckDB transform
- [x] #2 rfc_2822_ordinal has working transform or documented limitation
- [x] #3 `finetype check` passes
- [x] #4 Generators produce valid samples for both types
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add transform for julian_date: `strptime(concat('20', {col}), '%Y-%j')::DATE`
2. Add transform for rfc_2822_ordinal: `strptime(regexp_replace({col}, '(\\d+)(?:st|nd|rd|th)', '\\1'), '%d %b %Y %H:%M:%S %z')`
3. Both verified working in DuckDB. Generators already exist.
4. Run cargo test + cargo run -- check
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Filled transform stubs for two datetime types that had null transform AND null format_string.

Changes:
- `datetime.date.julian`: Added `strptime(concat('20', {col}), '%Y-%j')::DATE` — prepends '20' to 2-digit year before parsing
- `datetime.timestamp.rfc_2822_ordinal`: Added `strptime(regexp_replace({col}, '(\\d+)(?:st|nd|rd|th)', '\\1'), '%d %b %Y %H:%M:%S %z')` — strips ordinal suffixes before parsing

Tests: `cargo test` (258 passed), `cargo run -- check` (216/216 types, 10800/10800 samples). Both transforms verified in DuckDB CLI.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
