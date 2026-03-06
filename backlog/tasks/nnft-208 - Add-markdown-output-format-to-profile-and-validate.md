---
id: NNFT-208
title: Add markdown output format to profile and validate
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:14'
updated_date: '2026-03-06 04:59'
labels:
  - cli
  - output
milestone: m-8
dependencies: []
references:
  - crates/finetype-cli/src/main.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`--output markdown` for profile and validate commands. Clean pipe-separated tables suitable for docs, GitHub issues, and README embedding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 OutputFormat enum gains Markdown variant
- [x] #2 Profile markdown produces valid pipe-separated table
- [x] #3 Validate markdown produces quality report table
- [x] #4 Tables are well-formed with header separator row
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add Markdown variant to OutputFormat enum
2. Add OutputFormat::Markdown arm to profile command output
   - Pipe-separated table with Column | Type | Broad Type | Confidence columns
   - If validate: add Valid Rate column, show grade in header
   - Header separator row (---|---|---...)
3. Add OutputFormat::Markdown arm to validate command output
   - Quality report table: Label | Total | Valid | Invalid | Null | Validity Rate
4. Test with profile and validate commands
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added `--output markdown` format to profile and validate commands.

Changes:
- Added `Markdown` variant to `OutputFormat` enum
- Profile markdown: pipe-separated table with Column | Type | Broad Type | Confidence (+ Valid Rate | Quality when --validate)
- Validate markdown: quality report table with Label | Total | Valid | Invalid | Null | Validity Rate
- Non-profile/validate commands fall back to plain text for markdown format
- All tables well-formed with header separator rows, suitable for GitHub/docs

Tests:
- `cargo test` — 405 tests pass, 0 failures
- Manual test with profile (plain, validate, markdown combinations)
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
