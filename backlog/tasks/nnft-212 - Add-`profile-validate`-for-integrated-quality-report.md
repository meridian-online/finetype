---
id: NNFT-212
title: Add `profile --validate` for integrated quality report
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
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
- [ ] #1 `profile --validate` runs validation after classification
- [ ] #2 Plain output shows validity rate per column
- [ ] #3 JSON output includes quality object per column with valid/invalid/null counts
- [ ] #4 Columns without JSON Schema show null quality section
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
