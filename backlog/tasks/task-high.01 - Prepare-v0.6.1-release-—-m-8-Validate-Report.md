---
id: TASK-HIGH.01
title: Prepare v0.6.1 release — m-8 Validate & Report
status: To Do
assignee: []
created_date: '2026-03-06 05:31'
labels:
  - release-prep
dependencies: []
parent_task_id: TASK-HIGH
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.6.1 with m-8 Validate & Report milestone features: profile --validate, quality scores/grades, markdown output, quarantine samples, taxonomy fixes, actionability eval expansion. No model retrain needed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Version bumped to 0.6.1 in all Cargo.toml files
- [ ] #2 CHANGELOG.md updated with v0.6.1 entry
- [ ] #3 CI passes (fmt, clippy, test, taxonomy check)
- [ ] #4 GitHub release created via tag push
- [ ] #5 Homebrew tap updated automatically by CI
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
