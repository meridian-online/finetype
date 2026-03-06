---
id: NNFT-213
title: Per-column quality scores and overall quality grade
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:15'
updated_date: '2026-03-06 04:33'
labels:
  - core
  - quality
milestone: m-8
dependencies:
  - NNFT-212
references:
  - crates/finetype-core/src/lib.rs
  - crates/finetype-cli/src/main.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Composite quality metrics per column: type_conforming_rate, null_rate, completeness, quality_score. File-level grade (A/B/C/D/F) based on aggregate quality. Depends on NNFT-209 (profile --validate).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ColumnQualityScore struct in finetype-core
- [x] #2 JSON includes scores per column + overall file grade
- [x] #3 Grade thresholds: A≥95%, B≥85%, C≥70%, D≥50%, F<50%
- [x] #4 Unit tests for score calculation and grade assignment
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Create crates/finetype-core/src/quality.rs with:
   - ColumnQualityScore struct (type_conforming_rate, null_rate, completeness, quality_score)
   - FileQualityGrade enum (A/B/C/D/F) with Display and thresholds
   - compute_column_quality() function from validation stats
   - compute_file_grade() from column scores
2. Add pub mod quality to finetype-core/src/lib.rs with re-exports
3. Wire into CLI: update ColProfile to hold ColumnQualityScore, compute after validation
4. JSON output: add score fields per column + overall grade at file level
5. Plain output: show grade in summary line
6. Unit tests in quality.rs for score calculation and grade thresholds
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added per-column quality scores and file-level quality grades to the profile --validate output.

Changes:
- New `crates/finetype-core/src/quality.rs` module with `ColumnQualityScore` struct (type_conforming_rate, null_rate, completeness, quality_score) and `FileQualityGrade` enum (A≥95%, B≥85%, C≥70%, D≥50%, F<50%)
- `compute_column_quality()` and `compute_file_grade()` functions
- Re-exported from `finetype-core/src/lib.rs`
- CLI: replaced `ColQuality` with `ColProfileQuality` using `ColumnQualityScore`
- JSON output: quality object now includes type_conforming_rate, null_rate, completeness, quality_score; file-level grade field
- Plain output: summary line shows quality grade when --validate is active

Tests:
- 11 unit tests for score calculation and grade thresholds
- Full test suite: 405 tests pass, 0 failures
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
