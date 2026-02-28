---
id: NNFT-160
title: >-
  Retrain tiered model with CLDR-enriched data and evaluate (Phase 4 of
  CLDR-enriched retraining)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-27 23:07'
updated_date: '2026-02-28 01:10'
labels:
  - accuracy
  - cldr
  - phase-4
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 4 of the CLDR-Enriched Model Retraining plan (Option C).

Train tiered model on CLDR-enriched training data (training_cldr_v1.ndjson), evaluate against baselines, and make go/no-go decision.

Training command:
finetype train --model-type tiered --data training_cldr_v1.ndjson --output models/tiered-v2 --seed 42 --epochs 10 --batch-size 64

Auto-snapshot preserves current v0.3.0 model. entity_name and paragraph slot into existing tier paths.

Regression gates:
- Profile eval ≥ 116/120 label (current baseline)
- SOTAB ≥ 43.3% label / 68.3% domain (current baseline)
- Per-column diff analysis: flag any correct→incorrect changes

Known fragile columns: world_cities.name, utc_offset, codes_and_ids.cvv, people_directory.company

If regression: roll back to snapshot, analyze per-tier accuracy, create follow-up tasks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Tiered model trained on training_cldr_v1.ndjson with seed 42
- [x] #2 Auto-snapshot of existing v0.3.0 model created
- [x] #3 Profile eval run and compared against 116/120 baseline
- [x] #4 SOTAB eval run and compared against 43.3%/68.3% baseline
- [x] #5 Per-column diff analysis identifies any correct→incorrect regressions
- [x] #6 Go/no-go decision documented with evidence
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Train: finetype train --model-type tiered --data training_cldr_v1.ndjson --output models/tiered-v2 --seed 42 --epochs 10 --batch-size 64
2. Verify snapshot was created (auto-snapshot of existing model)
3. Run profile eval: make eval-report
4. Compare against baseline: 116/120 label, 118/120 domain
5. Run SOTAB eval: make eval-sotab-cli
6. Compare against baseline: 43.3% label, 68.3% domain
7. Per-column diff: identify any correct→incorrect regressions
8. Document go/no-go decision with evidence
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Training complete. Snapshot at models/tiered-v2.snapshot.20260227T231445Z

Key training accuracies:
- T0: 97.0% (15 broad types)
- T1 VARCHAR: 91.0% (22 categories)
- T2 VARCHAR/person: 92.0% (12 labels - now includes entity_name)
- T2 VARCHAR/text: 98.1% (7 labels - now includes paragraph)
- T2 DATE/date: 87.2% (17 labels)
- 17 T2 models at 100%, 5 models above 99%

## Profile Eval Results: REGRESSION — 107/120 vs 116/120 baseline

Rolled back to snapshot. Baseline 116/120 restored.

### New regressions (9 columns):

1. **URL→URI (×3)**: request_url, url, tracking_url — model learned URI type too aggressively, overcalls on URLs
2. **Country→nationality**: covid_timeseries.Country — person-geography confusion
3. **pressure_atm→latitude**: decimal numbers confused with coordinates
4. **airports.name→last_name**: person tier confusion (was full_name)
5. **utc_offset→iso_8601_offset**: datetime subtype confusion
6. **multilingual.country→entity_name**: entity classifier misfire on country names
7. **server_hostname→slug**: hostname confused with slug

### Existing misses (4 columns, same as baseline):
- codes_and_ids.swift_code → sedol
- countries.name → city
- people_directory.company → categorical
- books_catalog.publisher → categorical

### Go/No-Go: NO-GO. Model rolled back to v0.3.0 snapshot.

### Root cause analysis:
The CLDR-enriched data changed model behavior in several ways:
- URI vs URL: The model now includes both types and confuses them
- Location types: More diverse training data may have diluted location-specific signals
- Datetime: New datetime format diversity may have introduced confusion between similar types
- Person names: entity_name inclusion expanded VARCHAR/person to 12 labels (was 11), possibly reducing accuracy on existing types
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## CLDR-enriched retraining: REGRESSION, rolled back

Trained tiered model with CLDR-enriched data (84,500 samples, 169 types, seed 42). Auto-snapshot preserved v0.3.0 model.

### Training results

- T0: 97.0% (15 classes) — comparable to v0.3.0
- T1 VARCHAR: 91.0% (22 categories)
- T2 VARCHAR/person: 92.0% (12 labels, now includes entity_name)
- T2 VARCHAR/text: 98.1% (7 labels, now includes paragraph)
- T2 DATE/date: 87.2% (17 labels)
- 34 T2 models trained, 17 at 100%

### Profile eval: 107/120 (FAIL — baseline 116/120)

9 new regressions:
1. URL→URI ×3 — model overcalls URI on URL values
2. Country→nationality — person-geography confusion
3. pressure_atm→latitude — decimal/coordinate confusion
4. airports.name→last_name — person tier confusion
5. utc_offset→iso_8601_offset — datetime subtype confusion
6. multilingual.country→entity_name — entity classifier misfire
7. server_hostname→slug — text confusion

### Decision: NO-GO, rolled back

Model restored from snapshot. Baseline 116/120 confirmed after rollback.

### Root cause hypotheses

1. URI/URL confusion: Both types present in training, model can't distinguish — need URL→URI hierarchy or disambiguation rule
2. Training data diversity diluted type-specific signals — more samples per type needed, or targeted oversampling
3. Entity_name in VARCHAR/person expanded label space without enough disambiguating signal
4. Some regressions (latitude, slug) suggest training data quality issues in those generators

### Follow-up actions needed
- Increase samples per label (500→1000) or add oversampling for confused types
- Add URI/URL disambiguation rule or merge types
- Review generator quality for problematic types
- Consider training with priority ≥ 3 (exclude low-priority types that dilute signal)
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
