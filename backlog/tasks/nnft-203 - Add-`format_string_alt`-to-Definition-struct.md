---
id: NNFT-203
title: Add `format_string_alt` to Definition struct
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:14'
updated_date: '2026-03-06 04:26'
labels:
  - taxonomy
  - core
milestone: m-7
dependencies: []
references:
  - crates/finetype-core/src/taxonomy.rs
  - labels/definitions_datetime.yaml
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `format_string_alt` YAML field exists on `iso_8601` but is silently dropped because `Definition` struct has no corresponding field. Add the field, wire through taxonomy export and schema output.

This is foundational for m-7 — other tasks depend on the field being available in the struct.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Definition struct includes `format_string_alt: Option<String>`
- [x] #2 `taxonomy --full --output json` includes the field
- [x] #3 `schema` output includes it as extension field
- [x] #4 Tests pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add `format_string_alt: Option<String>` to Definition struct in taxonomy.rs
2. Wire into `definition_to_full_json()` in main.rs (line ~1709, after format_string)
3. Wire into `build_json_schema()` in main.rs (line ~1852, as x-format-string-alt extension field)
4. Run cargo test + cargo run -- check
Note: ACs #2 and #3 require main.rs changes — will coordinate with team lead since file ownership says not to touch main.rs.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added `format_string_alt: Option<String>` to the `Definition` struct in `crates/finetype-core/src/taxonomy.rs`. The field was already present in YAML (on `iso_8601`) but silently dropped during deserialization.

Changes:
- Added field to `Definition` struct with serde deserialization
- Wired into `definition_to_full_json()` for `taxonomy --full --output json`
- Wired into `build_json_schema()` as `x-format-string-alt` extension field

Tests: `cargo test` (258 passed), `cargo run -- check` (216/216 passing), verified JSON output includes field.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
