---
id: NNFT-214
title: Quarantine output with offending value samples
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:15'
updated_date: '2026-03-06 05:22'
labels:
  - cli
  - validation
milestone: m-8
dependencies:
  - NNFT-212
references:
  - crates/finetype-cli/src/main.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Up to 5 sample invalid values per column in `profile --validate` output. Helps users quickly understand why validation failed without inspecting the full dataset. Depends on NNFT-212 (profile --validate).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 JSON output includes invalid_samples array (up to 5) per column
- [x] #2 Markdown output shows 'Data Issues' section with samples
- [x] #3 Plain output shows top 3 invalid values when verbose
- [x] #4 Existing validate behavior unchanged when no invalid values
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add invalid_samples field to ColProfileQuality (Vec<String>, up to 5)
2. Collect quarantined values from validation result into invalid_samples
3. JSON output: add invalid_samples array per column when non-empty
4. Markdown output: add 'Data Issues' section listing columns with invalid samples
5. Plain output: show top 3 invalid values inline (only when there are invalids)
6. Verify no output changes when there are no invalid values
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added quarantine sample output to profile --validate for all three output formats.

Changes:
- Added `invalid_samples: Vec<String>` to `ColProfileQuality`, collecting up to 5 samples from quarantined values
- JSON output: `invalid_samples` array per column (only present when non-empty)
- Markdown output: 'Data Issues' section listing columns with invalid sample values as bullet items
- Plain output: top 3 invalid values shown inline below the column row with ⚠ prefix
- No output changes when all values are valid (invalid_samples key omitted from JSON, no Data Issues section)

Tests:
- `cargo test` — 405 tests pass, 0 failures
- Manual test with CSV containing valid + invalid values confirms correct samples in all formats
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
