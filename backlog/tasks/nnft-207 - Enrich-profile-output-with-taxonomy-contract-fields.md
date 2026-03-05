---
id: NNFT-207
title: Enrich profile output with taxonomy contract fields
status: To Do
assignee: []
created_date: '2026-03-04 20:14'
labels:
  - cli
  - profile
milestone: m-8
dependencies: []
references:
  - crates/finetype-cli/src/main.rs
  - crates/finetype-core/src/taxonomy.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Profile currently returns type/confidence/nulls. Add `broad_type`, `transform`, `format_string`, `is_generic` per column by looking up the predicted label in the taxonomy. This is foundational for the validate & report pipeline (m-8) and cross-milestone dependency for schema-for (m-10).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 JSON output includes broad_type, transform, format_string, is_generic per column
- [ ] #2 Plain output shows broad_type
- [ ] #3 CSV output includes new columns
- [ ] #4 Tests pass
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
