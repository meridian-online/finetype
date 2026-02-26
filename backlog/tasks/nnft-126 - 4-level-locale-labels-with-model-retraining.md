---
id: NNFT-126
title: 4-level locale labels with model retraining
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-25 03:31'
updated_date: '2026-02-26 00:01'
labels:
  - accuracy
  - locale
  - model-training
dependencies:
  - NNFT-121
  - NNFT-118
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generate training data with locale suffix labels (e.g., geography.address.postal_code.EN_US) and retrain CharCNN on the expanded ~484-class label set.

This is the high-risk, high-reward phase of locale intelligence:
1. Implement generate_all_localized() in generator.rs for locale-suffixed training data
2. Retrain CharCNN on expanded label set
3. Update inference pipeline to collapse 4-level predictions to 3-level user labels with locale metadata
4. Evaluate accuracy — no regressions allowed on profile eval

Depends on locale validation patterns (NNFT-118, NNFT-121) being proven in production first. May also benefit from CLDR data foundation (NNFT-060).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 4-level training data generated via generate_all_localized()
- [ ] #2 Model retrained on locale-expanded labels with acceptable accuracy
- [x] #3 Inference pipeline returns 3-level user label with locale as metadata field
- [ ] #4 No regression on profile eval
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan for NNFT-126: 4-Level Locale Labels with Model Retraining

### Background
- `generate_all_localized()` already produces 364 unique 4-level labels (260 locale-specific + 104 universal)
- Training pipeline currently strips locale suffixes back to 3-level at lines 97-119 of tiered_training.rs
- 18 type families have locale-specific variants across 6-17 locales each
- ColumnResult struct has no locale metadata field
- NNFT-134 finding: full_name overcall (3,086 SOTAB columns) needs model retraining

### Phase 1: Generate Training Data (~30 min)
1. Run `finetype generate --localized --samples 500` to produce ~182,000 training samples
2. Verify label distribution: each (type, locale) pair should have 500 samples
3. Current training.ndjson has only 364 samples (1 per label) — need substantial increase

### Phase 2: Training Infrastructure Changes (~2-3 hours)
4. Modify `tiered_training.rs` train_all() label normalization (lines 97-119):
   - T0/T1: Continue stripping locale suffix for routing (broad_type + category unchanged)
   - T2: Keep 4-level labels as class names (stop stripping at T2 level)
5. Update `train_tier2()` to use 4-level labels from samples instead of 3-level graph.types_for()
6. Update `prepare_batch()` (lines 469-478) to handle 4-level labels at T2
7. Update `build_graph_metadata()` to list 4-level labels in T2 tier_graph.json entries
8. Flat training: same approach — keep 4-level labels (for consistency, even though DuckDB extension uses flat)

### Phase 3: Inference Pipeline Updates (~2-3 hours)
9. Modify TieredClassifier: T2 models now return 4-level labels — inference engine extracts locale
10. Add `detected_locale: Option<String>` field to ColumnResult struct
11. Update column-level aggregation in classify_column():
    - Value predictions come as 4-level labels (e.g., postal_code.EN_US)
    - Group by 3-level prefix for majority vote (postal_code.EN_US + postal_code.FR_FR → postal_code)
    - Track locale distribution within winning 3-level type
    - Set detected_locale to dominant locale if locale-specific type wins
12. Update vote_distribution to show 3-level labels (user-facing)
13. Disambiguation rules: no changes needed (operate on 3-level labels after stripping)

### Phase 4: CLI and Output Updates (~1 hour)
14. Update CLI profile/infer output to show locale when detected
15. Update JSON detail output to include locale metadata
16. Ensure --batch mode handles locale metadata correctly

### Phase 5: Evaluation (~1 hour)
17. Profile eval: must stay at 70/74 (no regressions)
18. SOTAB eval: measure impact on domain accuracy (baseline 64.4%)
19. Verify locale detection on multilingual dataset

### Not In Scope (Separate Tasks)
- Entity name taxonomy classes for full_name overcall — separate task after retraining baseline
- CLDR data integration (NNFT-058/060) — can enrich locale patterns later
- DuckDB extension retrain — separate task (extension currently uses flat model)
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Phase 1 complete: Generated 182,000 training samples (364 labels × 500 per label).
Phase 2 complete: Training infrastructure updated — T0/T1 use 3-level labels, T2 uses 4-level locale labels.
Phase 3 complete: Inference pipeline updated — ColumnResult now has detected_locale field, vote aggregation collapses 4-level to 3-level.
184 tests pass (6 new locale suffix tests). Taxonomy check clean.
Training tiered-v3 model in progress.

Profile eval confirmed 70/74 with code changes against tiered-v2 (no regression from locale infrastructure). Training tiered-v3 running (PID 747375, ~25m CPU, ~28m wall time, still in training loop — no model files written yet).

Tiered-v3 model training complete (3h08m CPU). T2 accuracies:
- High (≥95%): text, offset, time, development, transportation, internet, INET, SMALLINT, DOUBLE/numeric, TIMESTAMP variants
- Good (80-95%): payment 91%, code 90%, contact 88%, medical 86%
- Low (locale expansion impact): address 71%, DATE/date 65%, DOUBLE/coord 68%, BIGINT/epoch 65%
- Challenge: location 55%, person 51% (94 4-level labels vs 13 in v2)

Profile eval REGRESSION: 70/74 → 67/74 with tiered-v3. 3 new errors:
1. tech_systems.os → full_name (was correct via header_hint_generic with v2 because phone_number was top vote; v3 puts full_name on top which is not in generic list)
2. books_catalog.url → uri (url vs uri confusion in expanded T2)
3. datetime_formats.utc_offset → iso_8601_offset (offset type confusion)

Root cause: T2 models with expanded 4-level labels (especially VARCHAR/person at 94 labels, 51% accuracy) change vote distributions, breaking header hint disambiguation. The 3-level vote aggregation works correctly but different per-value confusions shift which type gets plurality.

Reverted default symlink to tiered-v2.

Created decision-002 for locale detection strategy. Three options: (A) more training, (B) post-hoc validation-based locale, (C) hybrid tier. Awaiting decision before proceeding with AC #2 and #4.
<!-- SECTION:NOTES:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
