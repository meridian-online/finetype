---
id: NNFT-210
title: DDL-oriented taxonomy lookup API
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:14'
updated_date: '2026-03-06 07:12'
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
- [x] #1 DdlInfo struct in finetype-core with broad_type, transform, format_string, decompose fields
- [x] #2 ddl_info() lookup method on Taxonomy
- [x] #3 duckdb_type() mapping returns correct SQL type names
- [x] #4 Unit tests across all 7 domains
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Examine Definition struct in taxonomy.rs to understand broad_type field
2. Create DdlInfo struct with fields: duckdb_type, transform, format_string, format_string_alt, nullable, decompose
3. Implement duckdb_type() mapper from broad_type string to DuckDB SQL types (VARCHAR, TIMESTAMP, DOUBLE, DATE, BOOLEAN, BIGINT)
4. Implement Taxonomy::ddl_info(label) method that extracts fields from Definition
5. Add unit tests for all 7 domains (container, datetime, finance, geography, identity, representation, technology)
6. Run full test suite (cargo test) + taxonomy check (cargo run -- check)
7. Write final summary
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented DdlInfo struct and Taxonomy::ddl_info() lookup API for schema generation.

Changes:
- Added DdlInfo struct in finetype-core/src/taxonomy.rs with fields: duckdb_type, transform, format_string, format_string_alt, nullable, decompose
- Implemented DdlInfo::duckdb_type_from_broad_type() mapper from broad_type strings (VARCHAR, TIMESTAMP, DOUBLE, DATE, BOOLEAN, BIGINT, JSON, STRUCT, LIST) to DuckDB SQL type names
- Implemented Taxonomy::ddl_info(label) method extracting DDL metadata from any label definition
- Added 5 comprehensive unit tests: test_ddl_info_from_definition, test_ddl_info_missing_label, test_duckdb_type_mapping, test_ddl_info_across_domains

Tests:
- All 30 taxonomy tests pass (258 library tests total)
- Full taxonomy check passes: 216/216 definitions, all 7 domains passing
- Verified DdlInfo extraction works correctly across all 7 domains (datetime, identity, geography, finance, representation, container, technology)

Foundation for NNFT-220 (schema enrichment), NNFT-218 (schema-for), and NNFT-219 (Arrow output).
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
