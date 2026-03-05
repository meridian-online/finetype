---
id: NNFT-206
title: Fix currency broad_type mismatch + accounting notation
status: To Do
assignee: []
created_date: '2026-03-04 20:14'
labels:
  - taxonomy
  - finance
milestone: m-7
dependencies: []
references:
  - labels/definitions_finance.yaml
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`amount_us`/`amount_eu` declare `broad_type: VARCHAR` but their transforms produce DECIMAL — a mismatch that would break schema-for DDL generation. Also add accounting notation support for parenthesized negatives like `($1,234.56)`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 broad_type corrected to match transform output type
- [ ] #2 Validation accepts accounting notation e.g. ($1,234.56)
- [ ] #3 Generator produces accounting-notation samples
- [ ] #4 `finetype check` passes
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
