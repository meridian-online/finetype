---
id: NNFT-194
title: >-
  Post-retrain accuracy recovery for v0.5.3 — investigate profile eval
  regression
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-03 22:37'
updated_date: '2026-03-04 10:02'
labels:
  - accuracy
  - post-release
  - v0.5.3
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Investigate and resolve profile eval regression from v0.5.2 retrain.

**Regression summary:**
- v0.5.1 baseline: 117/119 (98.3% label), 119/119 (100% domain)
- v0.5.2 (after char-cnn-v10 retrain): 110/116 (94.8% label), 110/116 (94.8% domain)
- 3 columns removed from eval (unknown reason); 6 misclassifications

**Misclassifications (new in v0.5.2):**
1. utc_offset → excel_format (new)
2. ean → credit_card_number (new)
3. multilingual.name → region (new)
4. countries.sub-region → full_name (new)
5. countries.name → full_name (pre-existing, regression)
6. world_cities.name → full_name (new)

**Root cause:** CharCNN v10 retrain with 163-type taxonomy produced boundary shifts in decision space, not logic changes.

**Investigation approach:**
1. Compare CharCNN v9 vs v10 predictions on regression dataset (6 misclassifications)
2. Check if v9 predictions were correct and v10 regressed, or both wrong
3. Examine vote distributions for these columns — did masking/ranking change?
4. Determine if retrain is recoverable (model architecture) or requires taxonomy/pipeline adjustment
5. Consider per-type confidence thresholds or post-hoc rules to correct known misclassifications

**Non-blocking:** Profile eval at 94.8% is acceptable for release; regression documented in CHANGELOG. This task is follow-up for v0.5.3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Root cause analysis: identified 4 profile misclassifications and 1 actionability regression, traced to pipeline logic (Rule 17 guard, missing header hints, bare name ambiguity)
- [x] #2 Vote distribution analysis: confirmed Sense misrouting for bare name columns (temporal instead of geographic), CharCNN unable to distinguish geography subtypes from person names
- [x] #3 Mitigation implemented: 5 targeted fixes — Rule 17 guard removal, rfc_2822/rfc_3339/sql header hints, full_address hint, same-category hardcoded hint override, enhanced geography protection for low-confidence person-name hints
- [x] #4 Profile eval improved: 112/116 → 113/116 (97.4% label, 98.3% domain)
- [x] #5 Actionability improved: 95.4% → 97.9% (2810/2870 values)
- [x] #6 3 remaining misclassifications documented as CharCNN limitations requiring model retrain
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Fix 1: Remove Rule 17 UTC offset guard (column.rs ~line 1428-1435) — the disambiguate_utc_offset_override() already validates [+-]HH:MM at ≥80%, no top-label guard needed
2. Fix 2: Add rfc_2822/rfc_3339/sql_standard header hints before the generic timestamp catch-all (column.rs ~line 2099)
3. Fix 3: Remove bare h == "name" from full_name header hint conditions (column.rs lines 2084, 2087)
4. Fix 4: Investigate sports_events.venue GT — currently "name" → maps to full_name, but venue values are places ("Olympic Stadium"), not people
5. cargo build + cargo test -p finetype-model
6. Run make eval-report to verify profile + actionability improvements
7. Verify specific columns: utc_offset, countries.name, world_cities.name, rfc_2822
8. Update acceptance criteria and write final summary
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Root cause analysis revealed 4 profile misclassifications and 1 actionability regression were pipeline logic issues, not model quality problems:

1. **utc_offset → excel_format**: Rule 17 guard required top CharCNN vote to be datetime.time.* but v11 votes differently. The guard was unnecessary since disambiguate_utc_offset_override() already validates [+-]HH:MM at ≥80%.

2. **rfc_2822_timestamp → iso_8601**: header_hint() didn't distinguish specific timestamp formats. The generic `h.contains(\"timestamp\")` catch-all matched before any format-specific check. Fix: add rfc_2822/rfc_3339/sql_standard hints before catch-all. However, the Sense pipeline's conservative override conditions (only fires at confidence < 0.5 with hint in votes, or confidence < 0.3 fallback) meant the hint didn't apply when CharCNN confidently predicted iso_8601. Fix: added same-category hardcoded hint override that trusts headers within the same domain.category at ≤0.80 confidence.

3. **countries.name and world_cities.name → full_name**: Removing bare \"name\" → full_name hint exposed Sense misrouting (temporal/entity instead of geographic). CharCNN unmasked votes have no reliable geography signal for proper noun columns. Enhanced geography protection to check unmasked votes when confidence < 0.3 (indicating Sense mask likely wrong). Countries.name improved to \"region\" (correct domain) but still wrong type. World_cities.name remains full_name — no location type in unmasked votes.

4. **sports_events.venue GT**: Changed from \"name\" (maps to full_name) to \"entity name\" (maps to entity_name) since venue values are place names, not person names.

Additional fix: full_address header hint now distinguishes from street_address (h.contains(\"full\") guard).

Regression caught and fixed: same-category override was too aggressive without confidence guard, overriding correct full_address predictions at 0.99 confidence."
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Five targeted pipeline fixes to recover accuracy after CharCNN v11 retrain on locale-expanded data.

**Fixes applied (crates/finetype-model/src/column.rs):**
1. **Rule 17 UTC offset guard removed** — disambiguate_utc_offset_override() already validates [+-]HH:MM at ≥80%, the top-label guard was too narrow for v11 vote patterns
2. **Specific timestamp format header hints** — Added rfc_2822, rfc_3339, sql_standard matches before the generic date/timestamp/datetime catch-all (note: underscores become spaces after header normalization)
3. **Same-category hardcoded hint override** — When hardcoded header_hint() and prediction share the same domain.category (e.g., datetime.timestamp.*), trust the header at ≤0.80 confidence. Applied in both legacy and Sense pipelines.
4. **Enhanced geography protection** — For person-name hints with confidence < 0.3 (likely Sense misroute), check unmasked CharCNN votes for location types at ≥10%. Previously only checked masked vote distribution for generic predictions.
5. **full_address header hint** — h.contains(\"address\") && h.contains(\"full\") → geography.address.full_address, preventing same-category override from demoting to street_address

**Eval infrastructure (eval/datasets/manifest.csv):**
- sports_events.venue GT changed from \"name\" (→full_name) to \"entity name\" (→entity_name) — venue values are place names

**Results:**
- Profile: 112/116 → 113/116 (97.4% label, 98.3% domain)
- Actionability: 95.4% → 97.9% (2810/2870 values) — rfc_2822 now 100%
- 3 remaining misclassifications: countries.name→region (CharCNN can't distinguish country from region), sports_events.venue→city (expected entity_name), world_cities.name→full_name (expected city)

**Tests:**
- cargo test (258 model tests pass, including new header_hint tests for rfc_2822, rfc_3339, bare \"name\", display name, user name)
- cargo run -- check (163/163 taxonomy alignment)
- make eval-report (profile + actionability verified)
- cargo fmt + clippy clean"
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
