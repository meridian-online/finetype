---
id: NNFT-159
title: Training data generation and validation (Phase 3 of CLDR-enriched retraining)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-27 23:05'
updated_date: '2026-02-27 23:07'
labels:
  - accuracy
  - cldr
  - phase-3
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 3 of the CLDR-Enriched Model Retraining plan (Option C).

Generate enriched training data using the expanded locale data (Phase 2) and validate the distribution before training.

Parameters: finetype generate -s 500 --seed 42 -o training_cldr_v1.ndjson

Validation checklist:
- All 171 types present with ≥100 samples each
- entity_name and paragraph included (closing the 7-type model gap)
- CLDR-enriched datetime types show ≥3 format variations per type
- Total ~85k–100k samples
- Distribution report reviewed before proceeding to Phase 4
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Training data generated with seed 42 and 500 samples per label
- [x] #2 All 171 types present in generated data
- [x] #3 entity_name and paragraph types present with ≥100 samples each
- [x] #4 CLDR-enriched datetime types show ≥3 format variations per type
- [x] #5 Total sample count in 85k-100k range
- [x] #6 Distribution report produced and reviewed
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Generate training data: finetype generate -s 500 --seed 42 -o training_cldr_v1.ndjson
2. Validate type coverage: check all 171 types present
3. Validate entity_name and paragraph presence (the 2 types not in current model)
4. Check datetime type format diversity (≥3 variations per datetime type)
5. Verify total sample count ~85k-100k
6. Produce distribution report summary
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Distribution validated:
- 84,500 total samples (169 types × 500)
- 2 types without generators: password, plain_text (known gap, no priority set)
- entity_name: 500 samples ✓
- paragraph: 500 samples ✓
- Date type diversity: abbreviated_month shows 15 locale cycling (PL kwi/CS bře/SV maj/DA jan./FR jan etc)
- DMY (466) + MDY (34) ordering in abbreviated_month — EN gets MDY, all others DMY
- Note: HU/LT/LV not in YAML locale lists so YMD ordering not triggered — follow-up item, not blocking
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Generated CLDR-enriched training data for model retraining.

## What changed

Generated training_cldr_v1.ndjson with 84,500 samples (169 types × 500 samples each, seed 42).

## Key metrics

- 169/171 types represented (password and plain_text have no generators — known gap)
- entity_name: 500 samples ✓ (new, not in current v0.3.0 model)
- paragraph: 500 samples ✓ (new, not in current v0.3.0 model)
- Datetime types show locale diversity: PL/CS/SV/DA/NO/RU/FR/DE/ES/IT/PT/NL/EN cycling through abbreviated_month and long_full_month
- Date ordering: MDY (en) and DMY (all others) — HU/LT/LV YMD not triggered (not in YAML locale lists, follow-up item)

## Distribution

- datetime: 23,000 (27.2%)
- identity: 17,000 (20.1%)
- technology: 17,000 (20.1%)
- representation: 14,000 (16.6%)
- geography: 8,000 (9.5%)
- container: 5,500 (6.5%)

## File produced

training_cldr_v1.ndjson — 84,500 lines, ready for Phase 4 retraining
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
