---
id: NNFT-239
title: Retire `schema-for` command — merge Arrow output into `profile`
status: To Do
assignee:
  - '@nightingale'
created_date: '2026-03-07 01:18'
updated_date: '2026-03-07 01:18'
labels:
  - cli
  - cleanup
dependencies:
  - NNFT-238
references:
  - crates/finetype-cli/src/main.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
With the new `finetype load` command handling the analyst use case (CTAS output), `schema-for` has no remaining purpose:
- Its `-o json` output is a strict subset of `profile -o json` (which already has broad_type, transform, format_string, confidence, locale, quality)
- Its `-o plain` output is replaced by the superior `load` command
- Its `-o arrow` output should move to `profile`

Remove `schema-for` outright (no deprecation period — command is young, no known external consumers).

Depends on the `finetype load` task being created first so analysts have the replacement available.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 schema-for command removed from CLI (Commands enum, cmd_schema_for fn, SchemaOutputFormat enum)
- [ ] #2 finetype profile gains -o arrow output format (Arrow IPC JSON schema, moved from schema-for)
- [ ] #3 finetype profile -o arrow produces valid Arrow IPC JSON schema with proper type mappings
- [ ] #4 No references to schema-for remain in help text, README, or CLAUDE.md
- [ ] #5 cargo test passes after removal
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
