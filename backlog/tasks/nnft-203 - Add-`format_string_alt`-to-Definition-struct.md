---
id: NNFT-203
title: Add `format_string_alt` to Definition struct
status: To Do
assignee: []
created_date: '2026-03-04 20:14'
labels:
  - taxonomy
  - core
milestone: m-7
dependencies: []
references:
  - crates/finetype-core/src/taxonomy.rs
  - labels/definitions_datetime.yaml
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `format_string_alt` YAML field exists on `iso_8601` but is silently dropped because `Definition` struct has no corresponding field. Add the field, wire through taxonomy export and schema output.

This is foundational for m-7 — other tasks depend on the field being available in the struct.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Definition struct includes `format_string_alt: Option<String>`
- [ ] #2 `taxonomy --full --output json` includes the field
- [ ] #3 `schema` output includes it as extension field
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
