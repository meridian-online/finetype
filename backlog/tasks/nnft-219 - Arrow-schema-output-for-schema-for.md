---
id: NNFT-219
title: Arrow schema output for schema-for
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
updated_date: '2026-03-04 20:16'
labels:
  - cli
  - schema
  - arrow
milestone: m-10
dependencies:
  - NNFT-218
references:
  - crates/finetype-cli/src/main.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`--output arrow` on schema-for command. Maps broad_type → Arrow DataType (VARCHAR→Utf8, TIMESTAMP→Timestamp, etc.). Depends on NNFT-218 (schema-for command).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Valid Arrow schema JSON output
- [ ] #2 Covers all broad_types in taxonomy
- [ ] #3 Matches arrow::datatypes::Schema format
- [ ] #4 Roundtrip test: generate → parse → compare
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
