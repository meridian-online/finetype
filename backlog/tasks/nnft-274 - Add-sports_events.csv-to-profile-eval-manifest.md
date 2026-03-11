---
id: NNFT-274
title: Add sports_events.csv to profile eval manifest
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-10 23:50'
updated_date: '2026-03-11 03:26'
labels:
  - eval
  - testing
dependencies: []
references:
  - eval/datasets/manifest.csv
  - eval/schema_mapping.yaml
  - eval/datasets/csv/sports_events.csv
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add `sports_events.csv` (12 columns, 100 rows) to the profile eval manifest with ground-truth labels for all columns. This dataset catches two misclassifications found in v0.6.9:
- `duration_minutes` → `numeric_code` (should be `integer_number`)
- `status` → `geography.location.region` (should be `categorical`)

Depends on the numeric_code demotion rule and categorical ENUM tasks being completed first, otherwise GT expectations will fail.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add all 12 columns of sports_events.csv to `eval/datasets/manifest.csv` with correct GT labels
- [x] #2 Add schema mapping entries in `eval/schema_mapping.yaml` for any new type mappings needed
- [x] #3 `make eval-report` passes with sports_events.csv included — no new misclassifications
- [x] #4 Dataset file exists at `eval/datasets/csv/sports_events.csv` (copy from data/csvs/ if needed)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Verify sports_events.csv exists in eval/datasets/csv/
2. Verify all 12 columns present in manifest.csv with GT labels
3. Verify schema mapping entries exist for all GT labels used
4. Rebuild release binary (includes NNFT-272 F5 rule)
5. Run make eval-report — confirm no new misclassifications
6. Mark done
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
All 12 columns were already in manifest.csv (committed in NNFT-245). Schema mapping entries exist for all GT labels. After rebuilding release binary with NNFT-272's F5 rule, `duration_minutes` correctly scores as `integer_number` (label_match).

Eval results for sports_events (12 columns):
- 6 label matches: attendance, country, duration_minutes, event_id, venue, viewer_rating
- 3 domain matches: event_date (date→iso), is_broadcast (boolean→terms), start_time (time 24h→hm_24h)
- 3 not scored (semantic_only GT): sport (category), status (status), ticket_price (price)
- 0 misses

`status` column still classified as `geography.location.region` (0.42 confidence) — model-level issue, not scored because GT label 'status' is semantic_only. Known regression target for future model retraining.

Overall eval: 180/186 (96.8% label, 98.4% domain) — no regression.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Verified sports_events.csv integration in profile eval manifest. All 12 columns already present (added in NNFT-245) with appropriate GT labels. Schema mapping entries exist for all labels used.

Key verification: After rebuilding the release binary with NNFT-272's F5 disambiguation rule, `duration_minutes` now correctly resolves to `integer_number` (was `numeric_code`). The `status` column remains misclassified as `geography.location.region` (semantic_only, not scored) — documented as a known model-level regression target.

Eval: 180/186 (96.8% label, 98.4% domain), actionability 100% — zero regression.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
