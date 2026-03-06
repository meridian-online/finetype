---
id: NNFT-207
title: Enrich profile output with taxonomy contract fields
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:14'
updated_date: '2026-03-05 23:33'
labels:
  - cli
  - profile
milestone: m-8
dependencies: []
references:
  - crates/finetype-cli/src/main.rs
  - crates/finetype-core/src/taxonomy.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Profile currently returns type/confidence/nulls. Add `broad_type`, `transform`, `format_string`, `is_generic` per column by looking up the predicted label in the taxonomy. This is foundational for the validate & report pipeline (m-8) and cross-milestone dependency for schema-for (m-10).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 JSON output includes broad_type, transform, format_string, is_generic per column
- [x] #2 Plain output shows broad_type
- [x] #3 CSV output includes new columns
- [x] #4 Tests pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add is_generic field to ColumnResult in column.rs
2. Extend ColProfile struct in cmd_profile with broad_type, transform, format_string, is_generic
3. After classification, look up predicted label in taxonomy to populate new fields
4. Enrich JSON output with all four fields
5. Enrich plain output with broad_type column
6. Enrich CSV output with all four columns
7. Run cargo test + taxonomy check
8. Verify output formats manually
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Enriched all three profile output formats with taxonomy contract fields.

Changes:
- Added is_generic to ColumnResult (computed via finalize_is_generic helper on all return paths)
- JSON: broad_type, format_string, transform, is_generic per column
- Plain: BROAD column showing DuckDB target type (DATE, TIMESTAMP, VARCHAR, etc.)
- CSV: broad_type, format_string, transform, is_generic columns added

Verified: 381 tests pass, taxonomy check passes, clippy clean. All output formats manually verified.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
