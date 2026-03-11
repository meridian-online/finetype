---
id: NNFT-271
title: >-
  Release v0.6.9 — DuckDB fix, feature expansion, hierarchical head, financial
  hints
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-10 22:20'
updated_date: '2026-03-10 22:22'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.6.9 with changes since v0.6.8: NNFT-261 (DuckDB SMALLINT bug fix), NNFT-266 (column feature expansion + F3/F4 rules), NNFT-267 (hierarchical classification head), NNFT-268 (sibling-context attention module, inert), NNFT-270 (Sherlock-style features + financial header hints).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version bumped to 0.6.9 in Cargo.toml
- [x] #2 CHANGELOG.md updated with v0.6.9 entry
- [x] #3 All tests pass (cargo test + taxonomy check)
- [x] #4 Release tag created and CI triggered
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released v0.6.9. Tag pushed, CI triggered. Changelog covers all 6 shipped tasks (NNFT-261, 263, 266, 267, 268, 270) plus NNFT-269 discovery findings.
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
