---
id: NNFT-210
title: DDL-oriented taxonomy lookup API
status: To Do
assignee: []
created_date: '2026-03-04 20:14'
labels:
  - core
  - taxonomy
milestone: m-10
dependencies: []
references:
  - crates/finetype-core/src/taxonomy.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`Taxonomy::ddl_info(label) → Option<DdlInfo>` returning broad_type, transform, format_string, decompose. Plus `DdlInfo::duckdb_type()` mapping from broad_type string to DuckDB SQL type name.

This is foundational for schema-for and schema enrichment tasks in m-10.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DdlInfo struct in finetype-core with broad_type, transform, format_string, decompose fields
- [ ] #2 ddl_info() lookup method on Taxonomy
- [ ] #3 duckdb_type() mapping returns correct SQL type names
- [ ] #4 Unit tests across all 7 domains
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
