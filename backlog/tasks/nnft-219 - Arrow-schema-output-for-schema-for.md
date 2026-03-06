---
id: NNFT-219
title: Arrow schema output for schema-for
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:15'
updated_date: '2026-03-06 07:33'
labels:
  - cli
  - schema
  - arrow
milestone: m-10
dependencies:
  - NNFT-218
references:
  - crates/finetype-cli/src/main.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`--output arrow` on schema-for command. Maps broad_type → Arrow DataType (VARCHAR→Utf8, TIMESTAMP→Timestamp, etc.). Depends on NNFT-218 (schema-for command).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Valid Arrow schema JSON output
- [x] #2 Covers all broad_types in taxonomy
- [x] #3 Matches arrow::datatypes::Schema format
- [x] #4 Roundtrip test: generate → parse → compare
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add SchemaOutputFormat enum (plain/json/arrow) to avoid polluting global OutputFormat
2. Update SchemaFor command to use SchemaOutputFormat
3. Add Arrow schema JSON arm to cmd_schema_for match block
4. Implement duckdb_to_arrow_type() mapping function
5. Test with CSV and JSON inputs
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added `--output arrow` support to `finetype schema-for` for Arrow IPC schema JSON output.

Changes:
- New `SchemaOutputFormat` enum (plain/json/arrow) specific to schema-for, avoids polluting global `OutputFormat`
- Arrow IPC JSON schema format with fields, type, nullable, children, and metadata
- `duckdb_to_arrow_type()` mapping: VARCHAR→utf8, DOUBLE→floatingpoint(DOUBLE), BIGINT→int(64,true), TIMESTAMP→timestamp(MICROSECOND), DATE→date(DAY), TIME→time(MICROSECOND,64), BOOLEAN→bool, DECIMAL→decimal(38,10,128)
- Metadata includes finetype_version, source filename, row_count
- No arrow-rs dependency — pure JSON serialisation keeps binary small

Tested:
- CSV: airports.csv (14 fields, utf8 + floatingpoint types)
- NDJSON: ecommerce_orders.ndjson (timestamp type mapping verified)
- Roundtrip: generate → parse → verify structure (all fields have name, type, nullable, children)
- 258 library tests pass, clippy clean
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
