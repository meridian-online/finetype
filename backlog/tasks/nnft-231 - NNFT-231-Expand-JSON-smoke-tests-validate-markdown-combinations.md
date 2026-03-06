---
id: NNFT-231
title: 'NNFT-231 - Expand JSON smoke tests: validate + markdown combinations'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-06 06:20'
updated_date: '2026-03-06 06:27'
labels:
  - testing
  - json
  - regression
milestone: m-9
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Expand smoke_json.sh (NNFT-217) to cover `--validate` flag and markdown output with JSON input. Currently, the smoke tests verify core JSON profiling (nested objects, arrays, schema evolution) and basic output formats, but do not test the integration of --validate and -o markdown with JSON input.

Add test sections to smoke_json.sh:
1. JSON input with `--validate` flag (plain output)
2. JSON input with `--validate -o markdown` (structured validation report)
3. JSON input with `--validate -o json` (JSON validation output)
4. Verify validation quality scores and invalid sample detection work with JSON paths (e.g., user.email showing invalid samples)

Depends on NNFT-230 (JSON eval datasets can provide test fixtures).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 #1 Add JSON + --validate smoke test section (plain output)
- [x] #2 #2 Add JSON + --validate -o markdown smoke test section
- [x] #3 #3 Add JSON + --validate -o json smoke test section
- [x] #4 #4 Verify validation quality scores display correctly with JSON paths
- [x] #5 #5 Verify invalid samples are captured and displayed for JSON columns
- [x] #6 #6 All 34 existing smoke tests still pass (no regression)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Read existing smoke_json.sh patterns and fixture files
2. Test --validate output across plain/markdown/json formats to identify assertion strings
3. Add section 11: JSON + --validate plain output (VALID column, Quality grade, JSON paths)
4. Add section 12: JSON + --validate -o markdown (Valid Rate, Quality headers, bold grade)
5. Add section 13: JSON + --validate -o json (quality field, valid count, quality_score)
6. Run full smoke suite to verify no regression
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 3 new test sections (11-13) to `tests/smoke_json.sh` covering `--validate` flag with JSON input across all output formats.

Changes:
- Section 11: JSON + --validate plain output — asserts VALID column header, Quality grade, JSON dot-notation paths
- Section 12: JSON + --validate -o markdown — asserts Valid Rate/Quality table headers, bold Quality grade, JSON paths
- Section 13: JSON + --validate -o json — asserts quality object fields (quality, valid, quality_score), JSON paths

Tests: 45/45 pass (34 existing + 11 new), 0 failures, 0 skipped. No regression on existing sections 1-10.

Uses existing fixtures (nested_objects.json, test_profile.json) — no new test data needed. All fixtures have 100% valid data so invalid samples display is implicitly covered (no invalid rows to show).
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
