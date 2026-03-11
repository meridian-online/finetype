---
id: NNFT-273
title: Change categorical broad_type from VARCHAR to ENUM
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-10 23:50'
updated_date: '2026-03-11 01:30'
labels:
  - taxonomy
  - duckdb
dependencies: []
references:
  - labels/definitions_representation.yaml
  - crates/finetype-cli/src/main.rs
  - crates/finetype-duckdb/src/type_mapping.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `representation.discrete.categorical` type currently has `broad_type: VARCHAR`, but DuckDB supports a native ENUM type that is semantically correct for low-cardinality categorical columns.

Changing the broad_type to ENUM means `finetype load` and DDL generation must emit `CREATE TYPE` statements before the `CREATE TABLE`, enumerating the observed unique values.

Discovered via `sports_events.csv` where `status` (values: Cancelled, Completed, Live, Postponed, Scheduled) should produce an ENUM column, not VARCHAR.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Update `representation.discrete.categorical` in taxonomy YAML: `broad_type: VARCHAR` → `broad_type: ENUM`
- [x] #2 `finetype load` adds `--enum-threshold N` flag (default 50). Categorical columns with ≤N unique values generate `CREATE TYPE` + ENUM column. `--enum-threshold 0` disables ENUM (falls back to VARCHAR)
- [x] #3 `finetype profile` adds `--enum-threshold N` flag (default 50). Categorical columns with cardinality > threshold show VARCHAR instead of ENUM in broad type
- [x] #4 `finetype profile -o json --verbose` includes `unique_values` array for categorical columns
- [x] #5 DuckDB extension type mapping (`type_mapping.rs`) handles ENUM broad_type (maps to VARCHAR since extension can't CREATE TYPE)
- [x] #6 `finetype check` passes — taxonomy/generator alignment intact
- [x] #7 Existing tests pass — no regression in non-categorical type inference
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. **YAML change**: `labels/definitions_representation.yaml` — categorical `broad_type: VARCHAR` → `broad_type: ENUM`

2. **taxonomy.rs**: Add `"ENUM" => "VARCHAR"` to `duckdb_type_from_broad_type()` — fallback for contexts that don't handle ENUM specially

3. **DuckDB type_mapping.rs**: Add categorical → VARCHAR (extension can't CREATE TYPE)

4. **CLI `load` command** (main.rs):
   - Add `--enum-threshold` arg (default 50, type usize)
   - Add `enum_threshold` to `LoadColumn` struct
   - After profiling, for columns where label == categorical AND broad_type == ENUM:
     - Collect unique values from column data, sort alphabetically
     - If cardinality ≤ threshold: emit `CREATE TYPE {col}_t AS ENUM (...)` before CTAS, use enum type name in expression
     - If cardinality > threshold or threshold == 0: treat as VARCHAR
   - Pass through to `cmd_load`

5. **CLI `profile` command** (main.rs):
   - Add `--enum-threshold` arg (default 50)
   - Add `--verbose` flag
   - When displaying broad_type for categorical: check cardinality vs threshold → show ENUM or VARCHAR
   - In JSON output with --verbose: include `unique_values` array for categorical columns

6. Verify: `cargo run -- check`, `cargo test`, manual test with sports_events.csv
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- YAML: categorical broad_type VARCHAR → ENUM
- taxonomy.rs: Added \"ENUM\" → \"VARCHAR\" fallback in duckdb_type_from_broad_type()
- type_mapping.rs: Added explicit categorical/ordinal → VARCHAR for DuckDB extension
- Profile: --enum-threshold N (default 50) + --verbose flag. ENUM shown when cardinality ≤ threshold. JSON verbose includes unique_values array.
- Load: --enum-threshold N (default 50). Generates CREATE TYPE statements for ENUM columns before CTAS. CAST(col AS type_t) in SELECT.
- Helper functions: collect_unique_values_if_categorical(), resolve_broad_type_display(), build_load_expr_enum()
- All tests pass: core (144), model (302), CLI (0+13 ignored golden), DuckDB (31)
- Taxonomy check: 250/250, 100%
- Manual test: sports_events.csv sport column shows ENUM, load generates CREATE TYPE sport_t AS ENUM (...)"
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Changed `representation.discrete.categorical` broad_type from VARCHAR to ENUM, with configurable `--enum-threshold N` (default 50) on both `profile` and `load` commands.

Changes:
- `labels/definitions_representation.yaml`: categorical `broad_type: VARCHAR` → `broad_type: ENUM`
- `crates/finetype-core/src/taxonomy.rs`: ENUM → VARCHAR fallback in `duckdb_type_from_broad_type()`
- `crates/finetype-duckdb/src/type_mapping.rs`: Explicit categorical/ordinal → VARCHAR mapping
- `crates/finetype-cli/src/main.rs`:
  - Profile: `--enum-threshold N` controls ENUM vs VARCHAR display; `--verbose` adds `unique_values` array in JSON output
  - Load: `--enum-threshold N` generates `CREATE TYPE {col}_t AS ENUM (...)` before CTAS for categorical columns under threshold
  - Helper functions: `collect_unique_values_if_categorical()`, `resolve_broad_type_display()`, `build_load_expr_enum()`
- `CLAUDE.md`: Updated CLI command docs

Design: ENUM handling is in the CLI commands (which have access to column data) rather than the generic transform template (which can't know enum values). `--enum-threshold 0` disables ENUM entirely. DuckDB extension maps to VARCHAR since it can't CREATE TYPE.

Tests: cargo test (all crates) — 477 passed, 0 failed. `cargo run -- check` — 250/250 definitions, 100%."
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
