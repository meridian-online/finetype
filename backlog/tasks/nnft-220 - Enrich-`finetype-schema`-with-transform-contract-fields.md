---
id: NNFT-220
title: Enrich `finetype schema` with transform contract fields
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:15'
updated_date: '2026-03-06 07:22'
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
- [x] #1 Extension fields present in JSON Schema output
- [x] #2 All fields use `x-finetype-` prefix
- [x] #3 All datetime types include format_string in output
- [x] #4 Output remains valid JSON Schema Draft 2020-12
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add x-finetype-broad-type, x-finetype-transform, x-finetype-format-string fields to build_json_schema()
2. Export DdlInfo from finetype-core lib.rs
3. Verify output with multiple types
4. Run test suite
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added DDL extension fields to `finetype schema` JSON Schema output.

Changes:
- Added `x-finetype-broad-type`, `x-finetype-transform`, `x-finetype-format-string` to build_json_schema() in main.rs
- Exported DdlInfo from finetype-core lib.rs
- Existing `x-format-string-alt` kept without prefix for backwards compatibility
- Fields only emitted when present (no nulls in output)

Example output for datetime.timestamp.iso_8601:
  "x-finetype-broad-type": "TIMESTAMP"
  "x-finetype-transform": "strptime({col}, '%Y-%m-%dT%H:%M:%SZ')"\n  "x-finetype-format-string": "%Y-%m-%dT%H:%M:%SZ"\n  "x-format-string-alt": "%Y-%m-%dT%H:%M:%S.%gZ"\n\nTests:\n- All 258 library tests pass\n- Verified across datetime (TIMESTAMP), identity (VARCHAR), representation (DOUBLE) domains\n- Output remains valid JSON Schema Draft 2020-12
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
