---
id: NNFT-271
title: >-
  Release v0.6.9 — DuckDB fix, feature expansion, hierarchical head, financial
  hints
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-10 22:20'
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
- [ ] #1 Version bumped to 0.6.9 in Cargo.toml
- [ ] #2 CHANGELOG.md updated with v0.6.9 entry
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
