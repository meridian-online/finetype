---
id: NNFT-172
title: 'Eval: A/B comparison and regression testing for Sense pipeline'
status: To Do
assignee: []
created_date: '2026-02-28 23:07'
labels:
  - sense-sharpen
  - eval
dependencies:
  - NNFT-171
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run full evaluation suite comparing Sense pipeline vs legacy pipeline. Verify no regressions on profile eval, SOTAB, and actionability. Generate A/B diff report showing every prediction change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Profile eval label accuracy >= 116/120 (96.7%)
- [ ] #2 Profile eval domain accuracy >= 118/120 (98.3%)
- [ ] #3 SOTAB CTA label accuracy >= 43.6%
- [ ] #4 SOTAB CTA domain accuracy >= 68.6%
- [ ] #5 Actionability eval >= 98.5%
- [ ] #6 A/B diff report generated showing changes between Sense and legacy pipelines
- [ ] #7 Speed benchmark: mean column inference < 50ms
- [ ] #8 make ci passes
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
