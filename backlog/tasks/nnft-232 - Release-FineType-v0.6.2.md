---
id: NNFT-232
title: Release FineType v0.6.2
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-06 07:54'
updated_date: '2026-03-06 08:01'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.6.2 with m-10 Schema Export features (DdlInfo API, schema-for command, Arrow output, x-finetype-* fields)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Update version in Cargo.toml and Cargo.lock
- [x] #2 Run cargo test + check (all 258 tests pass, taxonomy check clean)
- [x] #3 Tag main with v0.6.2
- [ ] #4 Push tag to GitHub (CI builds + releases)
- [ ] #5 Verify Homebrew tap auto-update
- [ ] #6 Update crates.io
- [ ] #7 Verify finetype-duckdb community extension build passes
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
