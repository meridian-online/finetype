---
id: NNFT-208
title: Add markdown output format to profile and validate
status: To Do
assignee: []
created_date: '2026-03-04 20:14'
labels:
  - cli
  - output
milestone: m-8
dependencies: []
references:
  - crates/finetype-cli/src/main.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`--output markdown` for profile and validate commands. Clean pipe-separated tables suitable for docs, GitHub issues, and README embedding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 OutputFormat enum gains Markdown variant
- [ ] #2 Profile markdown produces valid pipe-separated table
- [ ] #3 Validate markdown produces quality report table
- [ ] #4 Tables are well-formed with header separator row
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
