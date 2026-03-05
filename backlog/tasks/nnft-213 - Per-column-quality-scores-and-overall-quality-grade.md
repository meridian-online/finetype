---
id: NNFT-213
title: Per-column quality scores and overall quality grade
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
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
- [ ] #1 ColumnQualityScore struct in finetype-core
- [ ] #2 JSON includes scores per column + overall file grade
- [ ] #3 Grade thresholds: A≥95%, B≥85%, C≥70%, D≥50%, F<50%
- [ ] #4 Unit tests for score calculation and grade assignment
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
