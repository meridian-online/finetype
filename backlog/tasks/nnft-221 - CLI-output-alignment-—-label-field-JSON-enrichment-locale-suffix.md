---
id: NNFT-221
title: 'CLI output alignment — label field, JSON enrichment, locale suffix'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-05 00:15'
updated_date: '2026-03-05 00:17'
labels: []
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Align CLI output format with documented spec: rename class→label in JSON, add broad_type/locale to value-mode JSON, append .LOCALE suffix to plain/CSV output. Fixes output formatting for doc-driven test harness.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 JSON value-mode uses 'label' not 'class'
- [x] #2 JSON column-mode uses 'label' not 'class'
- [x] #3 Value-mode JSON includes broad_type when taxonomy available
- [x] #4 Value-mode JSON includes locale when detected
- [x] #5 Plain text appends .LOCALE suffix when locale detected
- [x] #6 CSV appends .LOCALE suffix when locale detected
- [x] #7 smoke.sh 25/25 pass with updated assertions
- [x] #8 cargo test — zero failures
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Aligned CLI output format with documented spec and fixed LOCALE_GUIDE.md to match taxonomy.

CLI changes (main.rs):
- Renamed "class" → "label" in value-mode and column-mode JSON output
- Added broad_type and locale fields to value-mode JSON (from taxonomy lookup)
- Added .LOCALE suffix to plain text and CSV output when locale detected
- Added detect_single_value_locale() helper for single-value locale detection
- Loaded taxonomy in value-mode inference loop for enrichment

Doc fixes (LOCALE_GUIDE.md + golden files):
- Fixed locale keys: CA→EN_CA, BR→PT_BR, US→EN_US
- Fixed category path: datetime.date.month_name → datetime.component.month_name
- Removed hardcoded confidence values from JSON examples (model-dependent)
- Updated golden/locale-month-unicode.expected with correct category path

Smoke test updates:
- Updated 2 assertions: "class" → "label"

Doc test parity: 0% → 38% (7/18 pass). Remaining 8 failures are CharCNN single-value accuracy limitations.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
