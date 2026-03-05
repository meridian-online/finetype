---
id: NNFT-214
title: Quarantine output with offending value samples
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
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
- [ ] #1 JSON output includes invalid_samples array (up to 5) per column
- [ ] #2 Markdown output shows 'Data Issues' section with samples
- [ ] #3 Plain output shows top 3 invalid values when verbose
- [ ] #4 Existing validate behavior unchanged when no invalid values
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
