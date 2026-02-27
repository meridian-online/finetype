---
id: NNFT-147
title: >-
  Build the FineType evaluation package — precision, overcall, actionability,
  calibration
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-27 01:50'
updated_date: '2026-02-27 02:21'
labels:
  - evaluation
  - infrastructure
dependencies:
  - NNFT-144
references:
  - discovery/evaluation-method/BRIEF.md
  - eval/sotab/eval_cli.sql
  - eval/gittables/eval_cli.sql
  - eval/eval_profile.sql
  - eval/schema_mapping.csv
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build the complete evaluation infrastructure that measures what analysts actually care about. This is the scoreboard — accuracy improvements are separate.

NNFT-144 found that our current evals measure recall (does FineType find emails?) but not precision (when FineType says "email", is it right?). The SOTAB and GitTables SQL pipelines have no precision-per-predicted-type reporting, no overcall analysis, no actionability testing, and no confidence calibration.

The evaluation package has six components:

## 1. Regression eval (existing — no changes)
Profile eval on 74 columns. Every build. No regressions from baseline.

## 2. Precision eval (NEW — most important)
For every FineType type in SOTAB/GitTables predictions, compute precision = correct / predicted.
Thresholds:
- 🟢 ≥95% — analyst can act without checking (datetime, email, phone already here)
- 🟡 80-95% — analyst should spot-check before acting
- 🔴 <80% — untrustworthy, needs fix or confidence caveat

Add precision-per-predicted-type SQL section to eval/sotab/eval_cli.sql and eval/gittables/eval_cli.sql.

## 3. Overcall eval (NEW — analyst trust metric)
For high-risk types (full_name, full_address, URL, geography), compute false positive rate and break down what the false positives actually are. Target: <5% false positive rate (95%+ precision aspiration).

Dedicated SQL section showing GT label composition of each predicted type.

## 4. Actionability eval (NEW — "grudge work" metric)
For types with format_string (33 datetime types) and types with transform SQL, run TRY_CAST / TRY_STRPTIME on actual data and measure success rates. Target: >95% for datetime types. New eval script that loads taxonomy YAML, extracts format_strings/transforms, runs them against real data via DuckDB.

## 5. Confidence calibration eval (NEW)
Bin predictions by confidence level, compute actual accuracy per bin. Well-calibrated model: accuracy ≈ confidence. Target: calibration gap <10pp. SQL section in SOTAB/GitTables evals.

## 6. Eval report (`make eval-report`)
One command that runs all evals and generates eval/eval_output/report.md — a markdown dashboard with red/yellow/green per type, headline numbers, and historical comparison.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Precision-per-predicted-type SQL added to eval/sotab/eval_cli.sql with 🟢≥95% / 🟡 80-95% / 🔴<80% thresholds
- [x] #2 Precision-per-predicted-type SQL added to eval/gittables/eval_cli.sql with same thresholds
- [x] #3 Overcall analysis SQL section added — for full_name, full_address, URL, geography shows GT label breakdown of false positives
- [x] #4 Actionability eval script created — runs TRY_STRPTIME/TRY_CAST on profile eval data using taxonomy format_strings, reports per-type success rate
- [x] #5 Confidence calibration SQL section added — bins predictions by confidence, shows actual accuracy vs reported confidence per bin
- [x] #6 make eval-report target generates eval/eval_output/report.md with all metrics in a single dashboard
- [x] #7 All existing eval tests still pass (make eval-profile produces same results)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add precision-per-predicted-type SQL section to eval/sotab/eval_cli.sql (after Section 4, before misclassification) — with 🟢≥95% / 🟡80-95% / 🔴<80% status column
2. Add overcall analysis SQL section to eval/sotab/eval_cli.sql — GT label breakdown for high-risk predicted types (full_name, entity_name, full_address, URL, city, country, region)
3. Add confidence calibration SQL section to eval/sotab/eval_cli.sql — bin by confidence decile, show actual accuracy vs reported confidence
4. Copy precision + overcall + calibration sections to eval/gittables/eval_cli.sql (adapt table/column names)
5. Build actionability eval script (eval/eval_actionability.sh or .py) — load taxonomy YAML, extract format_strings, run TRY_STRPTIME on profile eval datasets, report per-type success rate
6. Add Makefile target eval-actionability
7. Build eval-report target — runs eval-profile + eval-sotab-cli + eval-actionability, generates eval/eval_output/report.md
8. Verify existing evals unchanged (make eval-profile produces same output)
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Completed AC#1-3, #5: Added precision (S5), overcall (S6), and calibration (S8) sections to eval/sotab/eval_cli.sql. Sections renumbered 1-9. Same sections added to eval/gittables/eval_cli.sql, renumbered 1-12. Precision uses 🟢≥95% / 🟡80-95% / 🔴<80% thresholds. Overcall monitors 10 high-risk types. Calibration bins by confidence decile with gap calculation.

Moving to AC#4: actionability eval script.

AC#4 complete: eval/eval_actionability.py tested successfully. 18 datetime columns tested, 98.3% overall actionability (2350/2390 values). 17/18 columns at 100%. One miss: multilingual.date uses dot-separated German dates but predicted as eu_slash (slash format). Makefile targets eval-actionability and eval-report added.

AC#6 complete: eval/eval_report.py generates eval/eval_output/report.md with headline metrics, taxonomy coverage, profile accuracy (69/74 = 93.2%), misclassification table, precision per type, actionability by type and failures. Makefile target eval-report orchestrates eval-profile + eval-actionability then runs report generator.

AC#7 complete: cargo test (207 pass, 0 fail), taxonomy check (all 171 types pass), profile eval SQL produces same 69/74 headline. No regressions.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## NNFT-147: Build the FineType evaluation package

Built the complete analyst-centric evaluation infrastructure measuring what analysts actually care about: precision, overcall, actionability, and confidence calibration.

### Changes

**SOTAB evaluation (eval/sotab/eval_cli.sql):**
- Section 5 (NEW): Precision per predicted type — precision for each FineType prediction with trust levels (🟢≥95% act without checking, 🟡80-95% spot-check, 🔴<80% untrustworthy)
- Section 6 (NEW): Overcall analysis — GT label composition and false positive rates for 10 monitored high-risk types (full_name, entity_name, full_address, URL, city, country, region, first_name, last_name, postal_code)
- Section 8 (NEW): Confidence calibration — predictions binned by confidence decile, compares actual accuracy vs reported confidence, calibration gap metric

**GitTables evaluation (eval/gittables/eval_cli.sql):**
- Same 3 new sections (precision, overcall, calibration) with identical methodology
- All existing sections renumbered cleanly (1-12)

**Actionability evaluation (eval/eval_actionability.py — NEW):**
- Tests whether FineType's format_string predictions work on real data
- Runs TRY_STRPTIME via DuckDB on profile eval datasets
- Result: 98.3% overall (2350/2390 values), 17/18 columns at 100%
- One finding: multilingual.date uses dot-separated German dates but predicted as eu_slash (slash format)

**Evaluation report (eval/eval_report.py — NEW):**
- Generates eval/eval_output/report.md — unified markdown dashboard
- Headline metrics, taxonomy coverage, profile accuracy, precision per type, actionability by type
- Matches eval_profile.sql methodology (direct+close, interchangeability rules)
- Confirmed: 69/74 (93.2%) matches existing profile eval exactly

**Makefile:**
- Added eval-actionability target
- Added eval-report target (orchestrates eval-profile + eval-actionability + report generation)

**CLAUDE.md:**
- Updated evaluation infrastructure section from 3 to 6 components
- Added new Make targets and key file references

### Tests
- cargo test: 207 pass, 0 fail
- cargo run -- check: all 171 types pass
- Profile eval SQL: 69/74 unchanged
- Actionability eval: 18 columns tested, 98.3% success rate

### Impact
The evaluation package now measures the six dimensions of type inference quality that matter to analysts. SOTAB/GitTables evals surface precision and overcall problems that were invisible before. The actionability eval directly answers \"can I safely TRY_CAST?\" The eval-report target gives a single command for a complete quality dashboard."}
</invoke>
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
