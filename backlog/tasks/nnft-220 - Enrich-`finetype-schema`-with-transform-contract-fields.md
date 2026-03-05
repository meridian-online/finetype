---
id: NNFT-220
title: Enrich `finetype schema` with transform contract fields
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
labels:
  - cli
  - schema
milestone: m-10
dependencies:
  - NNFT-210
references:
  - crates/finetype-cli/src/main.rs
  - crates/finetype-core/src/taxonomy.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add `x-finetype-broad-type`, `x-finetype-transform`, `x-finetype-format-string` extension fields to per-type JSON Schema output. These make the existing schema command actionable for DDL generation without the full schema-for pipeline.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extension fields present in JSON Schema output
- [ ] #2 All fields use `x-finetype-` prefix
- [ ] #3 All datetime types include format_string in output
- [ ] #4 Output remains valid JSON Schema Draft 2020-12
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
