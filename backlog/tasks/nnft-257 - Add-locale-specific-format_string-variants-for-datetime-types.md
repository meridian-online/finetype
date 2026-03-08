---
id: NNFT-257
title: Add locale-specific format_string variants for datetime types
status: To Do
assignee: []
created_date: '2026-03-08 06:04'
labels:
  - locale
  - datetime
  - taxonomy
dependencies: []
references:
  - data/cldr/cldr_date_patterns.tsv
  - data/cldr/cldr_time_patterns.tsv
  - labels/definitions_datetime.yaml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The taxonomy's format_string field for locale-specific datetime types (abbreviated_month, long_full_month, weekday_abbreviated_month, weekday_full_month) currently uses a single English-centric strptime pattern. Non-DuckDB consumers of the taxonomy (Python strptime, Rust chrono) would benefit from locale-specific format strings.

Note: DuckDB strptime only supports English month/day names, so this primarily serves external consumers of the JSON Schema / taxonomy export.

CLDR data for this mapping is already extracted in data/cldr/cldr_date_patterns.tsv and data/cldr/cldr_time_patterns.tsv (NNFT-157).

Follow-up from NNFT-058 (closed as superseded).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Locale-specific datetime types have per-locale format_string entries (or a new format_string_by_locale field)
- [ ] #2 Format strings are valid strptime patterns for the target locale's date ordering and separators
- [ ] #3 Existing format_string field remains unchanged for backward compatibility
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
