---
id: NNFT-212
title: Add `profile --validate` for integrated quality report
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:15'
updated_date: '2026-03-06 04:26'
labels:
  - cli
  - validation
milestone: m-8
dependencies:
  - NNFT-207
references:
  - crates/finetype-cli/src/main.rs
  - crates/finetype-core/src/validation.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run JSON Schema validation per column after profiling. Reuse existing `validate_column_for_label()`. Output quality stats: valid/invalid/null counts, validity rate, top error patterns. Depends on NNFT-207 (enriched profile output with taxonomy fields).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `profile --validate` runs validation after classification
- [x] #2 Plain output shows validity rate per column
- [x] #3 JSON output includes quality object per column with valid/invalid/null counts
- [x] #4 Columns without JSON Schema show null quality section
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add `--validate` flag to Profile command struct
2. Add quality fields to ColProfile struct (valid_count, invalid_count, null_count, validity_rate)
3. After column classification loop, if validate flag is set:
   - Load taxonomy and compile validators (reuse enrichment_taxonomy if available)
   - For each profile, call validate_column_for_label() with Quarantine strategy on the column's sample values
   - Store validation stats in ColProfile
4. Update Plain output: show validity rate per column when validate is active
5. Update JSON output: add quality object per column with valid/invalid/null/validity_rate
6. Columns without JSON Schema: show null quality section
7. Test with a temp CSV file
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added `--validate` flag to the `profile` command that runs JSON Schema validation per column after classification.

Changes:
- Added `--validate` CLI flag to Profile command
- Added `ColQuality` struct to hold per-column validation stats
- After classification loop, when `--validate` is set, loads taxonomy, compiles validators, and calls `validate_column_for_label()` with Quarantine strategy on each column's sample values
- Plain output: adds VALID column showing validity rate (e.g., `100.0%`, `75.0%`)
- JSON output: adds `quality` object per column with `valid`, `invalid`, `null` counts and `validity_rate`
- Columns without JSON Schema or unknown labels show null quality section

Tests:
- `cargo test` — 258 passed, 0 failed
- `cargo run -- check` — all 216 types passing
- Manual test with CSV containing valid + invalid values confirms correct counts
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
