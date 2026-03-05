---
id: NNFT-216
title: Wire JSON profiling into CLI profile command
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
updated_date: '2026-03-04 20:16'
labels:
  - cli
  - json
milestone: m-9
dependencies:
  - NNFT-215
references:
  - crates/finetype-cli/src/main.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`finetype profile --file data.json` auto-detects JSON by extension, flattens, feeds column classifier. Path-preserved output shows field paths like `users[].email`. Depends on NNFT-214 (NDJSON reader).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 JSON auto-detection by .json/.ndjson/.jsonl extension
- [ ] #2 NDJSON support via line-by-line reading
- [ ] #3 Field paths like `users[].email` preserved in output column names
- [ ] #4 All output formats (plain/json/csv/markdown) work with JSON input
- [ ] #5 Clear error message on malformed JSON input
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
