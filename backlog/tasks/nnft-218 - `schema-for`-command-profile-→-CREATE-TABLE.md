---
id: NNFT-218
title: '`schema-for` command: profile → CREATE TABLE'
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
labels:
  - cli
  - schema
milestone: m-10
dependencies:
  - NNFT-210
  - NNFT-207
references:
  - crates/finetype-cli/src/main.rs
  - crates/finetype-core/src/taxonomy.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`finetype schema-for data.csv` profiles the file, looks up DDL info per column, outputs `CREATE TABLE` with correct DuckDB types. Transform SQL included as comments. Depends on NNFT-210 (DDL lookup API) and NNFT-207 (enriched profile output).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Outputs valid DuckDB CREATE TABLE statement
- [ ] #2 Uses taxonomy broad_type for column types
- [ ] #3 `--table-name` flag overrides default table name
- [ ] #4 `--output json` returns structured schema object
- [ ] #5 Transform SQL included as inline comments
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
