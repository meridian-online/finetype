---
id: NNFT-239
title: Retire `schema-for` command — merge Arrow output into `profile`
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 01:18'
updated_date: '2026-03-07 22:16'
labels:
  - cli
  - cleanup
dependencies:
  - NNFT-238
references:
  - crates/finetype-cli/src/main.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
With the new `finetype load` command handling the analyst use case (CTAS output), `schema-for` has no remaining purpose:
- Its `-o json` output is a strict subset of `profile -o json` (which already has broad_type, transform, format_string, confidence, locale, quality)
- Its `-o plain` output is replaced by the superior `load` command
- Its `-o arrow` output should move to `profile`

Remove `schema-for` outright (no deprecation period — command is young, no known external consumers).

Depends on the `finetype load` task being created first so analysts have the replacement available.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 schema-for command removed from CLI (Commands enum, cmd_schema_for fn, SchemaOutputFormat enum)
- [x] #2 finetype profile gains -o arrow output format (Arrow IPC JSON schema, moved from schema-for)
- [x] #3 finetype profile -o arrow produces valid Arrow IPC JSON schema with proper type mappings
- [x] #4 No references to schema-for remain in help text, README, or CLAUDE.md
- [x] #5 cargo test passes after removal
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### 1. Move Arrow output format from schema-for to profile
- Add `arrow` variant to the existing `OutputFormat` enum (or create handling in cmd_profile)
- Port the `duckdb_to_arrow_type()` mapping and Arrow IPC JSON schema generation from cmd_schema_for into cmd_profile
- Profile already has all the column metadata needed (label, broad_type, etc.)

### 2. Remove schema-for command
- Remove `SchemaFor` variant from `Commands` enum
- Remove `SchemaOutputFormat` enum
- Remove `cmd_schema_for()` function
- Remove `Commands::SchemaFor` dispatch in main match
- Keep `duckdb_to_arrow_type()` and `format_column_name()` (still used by load/profile)

### 3. Update references
- Remove schema-for from CLAUDE.md CLI commands table
- Check for any other references (help text, README)

### 4. Verify
- cargo test + cargo run -- check
- cargo fmt + clippy
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Retired `schema-for` command and moved Arrow output to `profile`.

## What changed
- **Removed `schema-for` subcommand** entirely — Commands enum variant, dispatch, `cmd_schema_for()` function (~270 lines), and `SchemaOutputFormat` enum
- **Added `profile -o arrow`** — Arrow IPC JSON schema output format, using the existing `duckdb_to_arrow_type()` mapping (retained as a utility function)
- **Updated CLAUDE.md** — removed schema-for from CLI commands table, added arrow to profile's format list

## Why
With `finetype load` (NNFT-238) handling the analyst CTAS use case, `schema-for` had no remaining purpose:
- Its `-o plain` (CREATE TABLE DDL) is superseded by `load` (runnable CTAS with transforms)
- Its `-o json` is a strict subset of `profile -o json` (which includes confidence, locale, quality)
- Its `-o arrow` is now `profile -o arrow`

No deprecation period — the command was young with no known external consumers.

## Tests
- `cargo test` — all pass (254 tests)
- `cargo run -- check` — 250/250 taxonomy alignment
- `cargo fmt --check` + `cargo clippy` — clean
- `finetype schema-for` → \"unrecognized subcommand\" (confirmed removed)
- `finetype profile -o arrow` → valid Arrow IPC JSON schema output"
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
