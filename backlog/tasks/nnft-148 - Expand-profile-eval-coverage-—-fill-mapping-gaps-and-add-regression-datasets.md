---
id: NNFT-148
title: Expand profile eval coverage — fill mapping gaps and add regression datasets
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-27 02:23'
updated_date: '2026-02-27 02:41'
labels:
  - evaluation
  - infrastructure
dependencies:
  - NNFT-147
references:
  - eval/schema_mapping.yaml
  - eval/datasets/manifest.csv
  - eval/eval_output/report.md
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The profile eval (74 columns, 20 datasets) is our regression smoke test but has significant gaps:

1. **24 unmapped GT labels** — The manifest has ground truth for credit card number, CVV, EAN, first name, hash, HTTP status code, IATA, ICAO, IPv4, last name, MAC address, measurement unit, month name, NPI, occupation, port, SQL timestamp, street number, SWIFT code, time 24h, timestamp, user agent, UUID, decimal number. These all have FineType equivalents but no schema_mapping.yaml entries, so they're excluded from scoring.

2. **Thin type coverage** — Only 8 of 46 datetime types exercised. Many identity and technology types have zero profile eval coverage. 171 taxonomy types but only ~30 distinct predicted types in the eval.

3. **No regression datasets from accuracy fixes** — When we fix full_name overcall (NNFT-145), geography precision, or URL precision, there should be a regression dataset that exercises the fix.

This task fills the mapping gaps (cheap win) and establishes the pattern for growing coverage alongside accuracy work.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Schema mapping entries added for all 24 unmapped profile eval GT labels
- [x] #2 Profile eval scored pool increases from 74 to include newly-mapped columns
- [x] #3 At least 5 new datetime format types covered beyond the current 8 (iso, eu_slash, us_slash, iso_8601, sql_standard, hms_24h, hm_24h, iso_8601_microseconds)
- [x] #4 eval-report reflects expanded coverage without code changes
- [x] #5 Document the pattern for adding regression datasets alongside accuracy fixes (in eval/README.md or CLAUDE.md)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add 24 schema_mapping.yaml entries for unmapped profile eval GT labels
2. Regenerate schema_mapping.csv via make eval-mapping
3. Run eval-profile to get new scored pool numbers
4. Run eval-report to verify expanded coverage
5. Add datetime_formats_extended.csv dataset with 5+ new datetime types (eu_dot, rfc_2822, epoch, 12h time, abbreviated_month_date)
6. Update manifest.csv with new dataset columns and GT labels
7. Add corresponding schema_mapping entries for new datetime GT labels
8. Re-run eval-profile + eval-report to confirm expanded coverage
9. Document regression dataset pattern in eval/README.md
10. Verify no regressions (cargo test, taxonomy check)">
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1-2 complete: Added 24 schema_mapping.yaml entries (21 direct, 1 close, 2 partial). Scored pool: 74 → 112 columns. Label accuracy: 93.2% → 93.8% (105/112). Domain accuracy: 93.2% → 94.6% (106/112). Two new misses surfaced: cvv→postal_code, swift_code→sedol — both genuine misclassifications previously invisible.

- Created datetime_formats_extended.csv with 8 new datetime types: eu_dot, abbreviated_month, long_full_month, hm_12h, hms_12h, rfc_2822, american, european
- All 8 classified correctly (100% label + domain accuracy)
- All 8 format_strings parse at 100% success rate
- Scored pool: 112→120 columns, accuracy: 93.8%→94.2% label, 94.6%→95.0% domain
- Actionability: 18→26 columns tested, 8→16 datetime types, 98.3%→98.7% overall
- Fixed eval_report.py to use dynamic column/dataset count instead of hardcoded 74

- Added "Adding regression datasets" section to CLAUDE.md with 6-step pattern
- Updated profile eval numbers in CLAUDE.md (120 columns, 94.2% label, 95.0% domain)
- Updated actionability numbers (26 cols, 16 types, 98.7%)
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded profile eval coverage from 74 to 120 format-detectable columns across 21 datasets.

Changes:
- Added 24 schema_mapping.yaml entries for previously unmapped profile eval GT labels (payment, identity, medical, network, geographic, datetime, person types)
- Created datetime_formats_extended.csv with 8 new datetime types: eu_dot, abbreviated_month, long_full_month, hm_12h, hms_12h, rfc_2822, american, european
- Added 8 manifest entries + 8 schema mapping entries for the new datetime dataset
- Fixed eval_report.py to use dynamic column/dataset counts instead of hardcoded values
- Documented 6-step regression dataset pattern in CLAUDE.md
- Updated CLAUDE.md evaluation metrics to current numbers

Results:
- Label accuracy: 93.2% (69/74) → 94.2% (113/120)
- Domain accuracy: 93.2% (69/74) → 95.0% (114/120)
- Actionability: 98.3% (18 cols, 8 types) → 98.7% (26 cols, 16 types)
- All 8 new datetime types: 100% classification accuracy, 100% format_string parse rate
- Two previously hidden misclassifications surfaced: cvv→postal_code, swift_code→sedol

Tests: cargo test 305 pass, taxonomy check 171/171 pass, profile eval 113/120, actionability 25/26
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
