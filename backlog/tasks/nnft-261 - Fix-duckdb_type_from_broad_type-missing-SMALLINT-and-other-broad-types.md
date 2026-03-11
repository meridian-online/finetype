---
id: NNFT-261
title: Fix duckdb_type_from_broad_type missing SMALLINT and other broad types
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-08 10:04'
updated_date: '2026-03-08 10:05'
labels:
  - bugfix
  - load
dependencies: []
references:
  - crates/finetype-core/src/taxonomy.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `duckdb_type_from_broad_type()` match in `taxonomy.rs` was missing 5 broad types used in the taxonomy YAML definitions. Any unrecognised broad_type fell through to `VARCHAR`, causing the `load` command to skip CAST transforms.

Most visible: `datetime.component.year` has `broad_type: SMALLINT` but load output showed it as VARCHAR instead of casting to SMALLINT.

Root cause: the match only covered VARCHAR, DOUBLE, BIGINT, DECIMAL, DATE, TIMESTAMP, TIME, BOOLEAN, JSON, STRUCT, LIST — missing SMALLINT, TIMESTAMPTZ, INTERVAL, POINT, UUID.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 duckdb_type_from_broad_type handles SMALLINT, TIMESTAMPTZ, INTERVAL, POINT, UUID
- [x] #2 year column in datetime_formats.csv loads as SMALLINT not VARCHAR
- [x] #3 All existing tests pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Prevention note:** When adding new `broad_type` values to taxonomy YAML definitions, the corresponding match arm must also be added to `DdlInfo::duckdb_type_from_broad_type()` in `crates/finetype-core/src/taxonomy.rs`. The `_ => VARCHAR` fallback silently swallows unknown types — consider adding a `cargo test` or `finetype check` assertion that all broad_types in the taxonomy are covered by the match.

**Affected commands:** `load` (most visible — skips CAST), `schema` (incorrect DuckDB type in JSON Schema output). `profile` was unaffected because it reads `def.broad_type` directly from the YAML, not through this mapping function.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 5 missing broad_type mappings to `DdlInfo::duckdb_type_from_broad_type()`:

- SMALLINT → SMALLINT (fixes year column load)
- TIMESTAMPTZ → TIMESTAMP WITH TIME ZONE
- INTERVAL → INTERVAL
- POINT → POINT
- UUID → UUID

Single-site fix in `crates/finetype-core/src/taxonomy.rs`. All 281 tests pass.
<!-- SECTION:FINAL_SUMMARY:END -->
