---
id: NNFT-132
title: Phone validation investigation — SOTAB telephone demotion to categorical
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-25 09:17'
updated_date: '2026-02-25 11:40'
labels:
  - accuracy
  - validation
  - discovery
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
634 SOTAB telephone columns are being demoted from phone_number to categorical by attractor demotion rules. Actual values include formats like (661) 284-3600, 05 61 85 61 48, 07584674902.

This is a discovery spike to determine:
- Which phone formats in SOTAB aren't covered by our 14 locale patterns?
- Is it a pattern-coverage gap (add more locale patterns) or something deeper (cardinality demotion firing inappropriately)?
- What's the fix: more locale patterns, relaxed validation, or cardinality threshold adjustment?

Time budget: ~2-4 hours investigation, produce written finding with data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Sample SOTAB phone columns analysed — identify which formats fail current locale validation
- [x] #2 Root cause determined: pattern gap vs cardinality demotion vs confidence threshold
- [x] #3 Written finding with data: which fix path gives the most recovery
- [x] #4 Follow-up implementation task created if fix is viable
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Phase 1: Data Collection & Error Categorisation (complete)
1. Extract all SOTAB telephone-GT columns (486 total) from cli_predictions.csv
2. Categorise by demotion mechanism: cardinality (254), model-level (213), validation (14), confidence (5)
3. Sample actual phone values from column_values.parquet for each category

Phase 2: Root Cause Analysis (complete)
4. Primary cause: Signal 3 cardinality demotion fires unconditionally (254/273 = 93%)
5. Universal phone pattern is too permissive — matches anything with digits, not real validation
6. Locale patterns confirm only 23/254 columns — honest signal of thin coverage
7. Secondary: 14 cols have genuine pattern gaps (extensions, German (0) prefix, ZA locale)

Phase 3: Validation Precision Architecture
8. Split validation_confirmed into locale_confirmed (strong) and format_compatible (weak)
   - For locale_specific types: only locale patterns set locale_confirmed
   - Universal validation remains as format gate (can reject via Signal 1, cannot confirm)
9. Gate Signals 2 and 3 with locale_confirmed for locale_specific types
   - Signal 2: if !locale_confirmed && majority_fraction < 0.85 → demote
   - Signal 3: if is_text && !locale_confirmed && unique <= 20 → demote
10. Preserve current behaviour for non-locale types (validation_confirmed unchanged)
11. Add Validation Precision Principle to CLAUDE.md Decided Items

Phase 4: Written Finding & Follow-up
12. Document analysis and architectural decision in task notes
13. Create follow-up task: expand phone locale patterns (extensions, more countries)
14. Create follow-up task: expand postal_code locale patterns
15. Update memory with investigation results and precision principle
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Investigation Data (Phase 1-2 complete)

### Error distribution for 486 SOTAB telephone columns:
- 254 (52%): Signal 3 cardinality demotion — model predicts phone_number, low unique values
- 213 (44%): Model never predicts phone — ISSN (70), full_address (24), alphanumeric_id (21), etc.
- 14 (3%): Signal 1 validation failure — format gaps (extensions, German (0), ZA)
- 5 (1%): Signal 2 confidence threshold

### Key findings:
- 235/254 cardinality-demoted columns have confidence 1.0
- 254/254 pass universal phone validation (but universal pattern is too permissive)
- Only 23/254 pass any locale-specific pattern at >50%
- Zero false positive regressions — all 365 cardinality-demoted phone_number predictions are actual phone/fax
- Signal 3 fires unconditionally (line 1771) — no validation_confirmed or locale_confirmed guard

### Root cause:
Primary: Signal 3 has no locale_confirmed guard (code architecture gap)
Secondary: Only 14 locale patterns — thin coverage for international phone formats
Underlying: Universal phone validation too permissive to use as confirmation signal

### Decision: Validation Precision Principle (approved by Hugh)
- For locale_specific types, only locale patterns count as confirmation
- Universal validation can reject but cannot confirm
- Expanding locale coverage is the accuracy lever, not weakening gates
- Added to CLAUDE.md Decided Items as item 14

## Code Changes (Phase 3 complete)

Implemented validation precision in disambiguate_attractor_demotion:
1. Hoisted locale_confirmed to function scope, added has_locale_validators flag
2. Stopped setting validation_confirmed from locale confirmation (independent signals)
3. Signal 2: gated by locale_confirmed for locale-specific types, validation_confirmed for others
4. Signal 3: gated by locale_confirmed — skip cardinality demotion when locale patterns confirm

Added 3 new tests:
- test_attractor_locale_confirmed_skips_cardinality: phone with locale match + low cardinality → no demotion
- test_attractor_universal_only_does_not_confirm_locale_type: phone with only universal match + low confidence → demote
- test_attractor_first_name_cardinality_unchanged: first_name cardinality demotion unaffected (regression guard)

All 174 tests pass. Profile eval: 70/74 unchanged.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Investigated 486 SOTAB telephone columns to determine why phone_number predictions are being demoted to categorical. Found the root cause is NOT pattern-coverage gaps (original hypothesis) but Signal 3 (cardinality demotion) firing unconditionally for text attractors, ignoring locale validation confirmation.

Key finding: the universal phone validation pattern (^[+]?[0-9\s()\-\.]+$) is too permissive to serve as a confirmation signal — it matches nearly anything with digits. This led to establishing the Validation Precision Principle: for locale-specific types, only locale patterns can confirm; universal validation can reject but cannot confirm.

Code changes in disambiguate_attractor_demotion (column.rs):
- Split validation tracking into locale_confirmed (strong, from locale patterns) and validation_confirmed (weak, from universal patterns)
- Added has_locale_validators flag to distinguish locale-specific types
- Signal 2: gated by locale_confirmed for locale-specific types, validation_confirmed for others
- Signal 3: gated by locale_confirmed — skip cardinality demotion when locale patterns confirm
- 3 new tests: locale-confirmed phone skips cardinality, universal-only does not confirm locale type, first_name cardinality unchanged (regression guard)

CLAUDE.md updated:
- Added Precision Principle section under Noon Pillars
- Added Decided Item 14 (validation precision for locale-specific types)
- Updated Decided Item 11 and architecture section for consistency

Impact: 23 SOTAB telephone columns immediately rescued (locale-confirmed). Remaining 231 require expanded locale patterns (NNFT-136). Profile eval unchanged at 70/74. Decision record decision-001 created.

Follow-up: NNFT-136 — expand phone locale patterns (extensions, ZA locale, German trunk prefix).
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
