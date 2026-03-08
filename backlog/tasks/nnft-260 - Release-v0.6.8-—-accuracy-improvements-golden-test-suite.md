---
id: NNFT-260
title: 'Release v0.6.8 — accuracy improvements, golden test suite'
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-08 09:39'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.6.8 with NNFT-254 accuracy improvements and NNFT-258 golden test suite.

Changes since v0.6.7:
- NNFT-254: ~30 new header hints, cross-domain override, 7 substring bug fixes. Profile: 178/186 → 179/186 (96.2% label, 98.4% domain)
- NNFT-258: 13 golden integration tests covering profile, load, taxonomy, schema commands
- NNFT-259: Discovery task created for context-aware header classifier (no code changes)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Version bumped to 0.6.8 in Cargo.toml
- [ ] #2 CHANGELOG.md updated with v0.6.8 entry
- [ ] #3 All tests pass (cargo test + taxonomy check)
- [ ] #4 Release tag created and CI triggered
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
