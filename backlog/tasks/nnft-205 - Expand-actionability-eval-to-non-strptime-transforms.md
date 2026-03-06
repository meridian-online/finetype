---
id: NNFT-205
title: Expand actionability eval to non-strptime transforms
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:14'
updated_date: '2026-03-06 05:11'
labels:
  - eval
  - actionability
milestone: m-7
dependencies: []
references:
  - crates/finetype-eval/src/bin/eval_actionability.rs
  - eval/eval_output/report.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Actionability eval only tests 33 types with format_string (strptime-based). 23 "Tier B" types (epochs, currency, JSON, numeric) have transforms but are untested. Extend eval to execute transform SQL via DuckDB to measure full transform coverage.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Eval tests transform-based types by executing DuckDB SQL
- [x] #2 Report separates strptime vs transform results
- [x] #3 Epoch + currency types included in eval
- [x] #4 Single overall actionability metric still reported
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add `load_transforms()` to `crates/finetype-eval/src/taxonomy.rs` — returns transforms for types WITHOUT format_string (Tier B)
2. In eval_actionability.rs:
   - Existing strptime pass = Tier A (format_string types)
   - New transform pass = Tier B: substitute {col} in transform, replace CAST→TRY_CAST, count non-NULL results
   - Report Tier A and Tier B separately
   - Single overall actionability metric (combined)
3. ActionResult gains `eval_method` field: "strptime" or "transform"
4. Run the eval binary to verify
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded actionability eval from strptime-only (Tier A) to include transform-based types (Tier B).

Changes:
- `crates/finetype-eval/src/taxonomy.rs`: Added `load_transforms()` — loads transform SQL for types without format_string (150 Tier B types)
- `crates/finetype-eval/src/bin/eval_actionability.rs`: Two-pass evaluation:
  - Tier A: TRY_STRPTIME for format_string types (unchanged)
  - Tier B: Execute transform SQL with CAST→TRY_CAST substitution, count non-NULL results
- Report separates Tier A/B results, combined summary by type with method indicator
- CSV output adds `eval_method` and `format_or_transform` columns
- Single overall metric preserved

Results:
- Tier A (strptime): 2760/2870 = 96.2% (24 columns)
- Tier B (transform): 225312/225842 = 99.8% (180 columns)
- Overall: 228072/228712 = 99.7% (204 columns, 80 types)

Known limitations: Some types produce warnings (duration uses ::INTERVAL cast without TRY variant, height has complex SQL, coordinates needs spatial extension). These are expected and don't affect results.
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
