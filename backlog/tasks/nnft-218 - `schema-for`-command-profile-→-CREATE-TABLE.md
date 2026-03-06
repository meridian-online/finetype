---
id: NNFT-218
title: '`schema-for` command: profile → CREATE TABLE'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:15'
updated_date: '2026-03-06 07:28'
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
- [x] #1 Outputs valid DuckDB CREATE TABLE statement
- [x] #2 Uses taxonomy broad_type for column types
- [x] #3 `--table-name` flag overrides default table name
- [x] #4 `--output json` returns structured schema object
- [x] #5 Transform SQL included as inline comments
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add SchemaFor command variant to CLI enum
2. Add dispatch arm for SchemaFor
3. Implement cmd_schema_for() reusing profile pipeline (classifier setup, CSV/JSON readers)
4. Output SQL CREATE TABLE (plain) and structured JSON (--output json)
5. Add sanitise_identifier() and format_column_name() helpers
6. Test with CSV, NDJSON, --table-name, --output json
7. Run test suite + clippy
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented `finetype schema-for` command that profiles a file and generates DuckDB CREATE TABLE DDL.

Changes:
- New `SchemaFor` CLI subcommand with flags: --file, --table-name, --model, --output, --sample-size, --delimiter, --no-header-hint, --model-type, --sharp-only
- `cmd_schema_for()` reuses full Sense→Sharpen profile pipeline (same classifier setup as cmd_profile)
- Supports CSV, JSON, NDJSON, JSONL input formats via existing readers
- SQL output (default): CREATE TABLE with aligned columns, DuckDB types, inline comments showing label + transform
- JSON output (--output json): structured object with table_name, columns array, row/column counts
- Table name derived from filename stem (sanitised), overridable via --table-name
- Column names auto-quoted when containing dots, spaces, hyphens, brackets, or starting with digit
- Generic predictions (is_generic=true) default to VARCHAR per Noon design principle
- Added sanitise_identifier() and format_column_name() helpers

Tested:
- CSV: airports.csv (14 cols), countries.csv (11 cols with hyphenated names)
- NDJSON: ecommerce_orders.ndjson (10 cols, JSON paths)
- --table-name override, --output json format
- All 258 tests pass, clippy clean
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
