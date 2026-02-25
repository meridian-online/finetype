---
id: NNFT-131
title: Disambiguation rule sprint — Duration/TLD/schema mapping fixes
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-25 09:16'
updated_date: '2026-02-25 09:53'
labels:
  - accuracy
  - disambiguation
  - eval
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Post-NNFT-130 evaluation identified 7,987 format-detectable errors in SOTAB. Three rule-based fixes address ~1,200 columns:

1. **Duration vs SEDOL** (218 cols) — ISO 8601 durations (PT20M, PT1H) misclassified as SEDOL stock codes. New disambiguation rule.
2. **TLD attractor** (252 cols) — Small decimals (1.0, 5.0) misclassified as top_level_domain. Add TLD to CODE_ATTRACTORS.
3. **Schema mapping** (736 cols) — SOTAB DateTime/Date variants scored as wrong when FineType is actually correct (iso_8601_offset for offset timestamps, long_full_month for spelled-out dates).

Expected impact: SOTAB format-detectable domain 54.8% → ~57%, label accuracy 30.5% → ~37%.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 New disambiguation rule: Duration override fires when top vote is SEDOL and values match ISO 8601 duration pattern (P prefix + time components)
- [x] #2 TLD added to CODE_ATTRACTORS — numeric values fail TLD validation and get demoted
- [x] #3 SOTAB schema mapping updated to accept DateTime variants (iso_8601_offset, sql_standard) and Date variants (long_full_month)
- [x] #4 Profile eval remains 70/74 (no regressions)
- [x] #5 cargo test passes with no regressions
- [x] #6 SOTAB CLI eval re-run shows measurable improvement
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add Duration vs SEDOL disambiguation rule (Rule 15) in column.rs
   - Gate on top vote == identity.payment.sedol
   - Check >=50% values match ISO 8601 duration pattern (P prefix + time components)
   - Insert between Rule 13 (SI number) and Rule 14 (attractor demotion)
2. Add TLD to CODE_ATTRACTORS in column.rs
   - Existing validation-based demotion handles numeric values automatically
3. Update SOTAB schema mapping to accept DateTime/Date variants
   - Add variant rows to sotab_schema_mapping.csv
   - Update eval_cli.sql with best-match join logic
4. Add unit tests for duration disambiguation
5. Run cargo test, cargo run -- check, verify profile eval
6. Create future investigation tasks (text overcall, numeric age)
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
v0.3.0 → post-sprint results:
- Duration override: 235 cols fired (expected ~218)
- TLD attractor demotion: 355 cols fired (expected ~252)
- Schema mapping: DateTime/Date variants now accepted
- Profile eval: 70/74 unchanged
- SOTAB format-detectable: 30.5% → 39.5% label (+9.0pp), 54.8% → 59.5% domain (+4.7pp)
- All 171 tests pass, taxonomy check passes
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Disambiguation rule sprint targeting 3 accuracy improvements identified by NNFT-130 error analysis.

## Changes

### 1. Duration vs SEDOL override (Rule 14)
- New `disambiguate_duration_override()` function in `column.rs`
- Gates on top vote == `identity.payment.sedol`
- Checks >=50% of values match ISO 8601 duration pattern (P prefix + Y/M/W/D/T/H/S)
- Overrides to `datetime.duration.iso_8601`
- Inserted before attractor demotion (now Rule 15) to prevent SEDOL being demoted to alphanumeric_id
- Impact: 235 SOTAB columns recovered

### 2. TLD attractor demotion
- Added `technology.internet.top_level_domain` to CODE_ATTRACTORS
- Numeric values (1.0, 5.0) fail TLD alphabetic validation → demoted automatically
- Impact: 355 SOTAB columns fired TLD demotion

### 3. SOTAB schema mapping expansion
- Added variant rows for DateTime: iso_8601_offset, sql_standard
- Added variant rows for Date: long_full_month, us_slash, eu_slash, short_mdy, short_dmy
- Updated eval_cli.sql with best-match join logic (exact variant match preferred, primary mapping as fallback)
- Impact: ~500+ columns now scored correctly at label level

## Results
- SOTAB format-detectable: 30.5% → 39.5% label (+9.0pp), 54.8% → 59.5% domain (+4.7pp)
- Profile eval: 70/74 unchanged (no regressions)
- All 171 tests pass, taxonomy check passes
- 6 new unit tests for duration disambiguation

## Future tasks created
- NNFT-134: Text overcall investigation (address/name false positives)
- NNFT-135: Numeric age/integer disambiguation investigation
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
