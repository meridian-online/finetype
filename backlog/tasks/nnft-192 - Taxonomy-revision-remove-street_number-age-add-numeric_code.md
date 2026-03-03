---
id: NNFT-192
title: 'Taxonomy revision: remove street_number & age, add numeric_code'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-03 10:56'
updated_date: '2026-03-03 13:04'
labels:
  - taxonomy
  - feature
dependencies: []
references:
  - labels/definitions_geography.yaml
  - labels/definitions_identity.yaml
  - labels/definitions_representation.yaml
  - crates/finetype-model/src/label_category_map.rs
  - crates/finetype-model/src/column.rs
  - discovery/taxonomy-revision/RESPONSE_CLAUDE.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Three taxonomy changes in a single retrain cycle:

**Removals:**
1. `geography.address.street_number` — Values like "123", "456" are indistinguishable from regular integers. Validation pattern matches plain integers, causing false positives. Previous demotion rules (NNFT-117) were a workaround; removal is the clean fix. Values fall to `integer_number` or `alphanumeric_id` naturally.

2. `identity.person.age` — `CAST({col} AS SMALLINT)` is effectively the same as `integer_number`. Validation `^[0-9]{1,3}$` range 0–150 matches any small integer. 205 SOTAB columns of generic numbers misclassified as age at 0.995 confidence (NNFT-135). Removal resolves NNFT-135 entirely.

**Addition:**
3. `representation.identifier.numeric_code` — All-digit strings where the digit pattern matters: consistent length, leading zeros valid. Covers country numeric codes (840, 036), NAICS/SIC industry codes, product codes, FIPS codes. The key value is *preventing* integer cast — broad_type is VARCHAR. Should only fire where geography types (postal_code) or phone_number don't apply. Addresses the #2 analyst frustration from taxonomy revision research (leading zeros stripped).

**Net effect:** 164 → 163 types. Requires model retrain.

**Context:**
- Discovery research: `discovery/taxonomy-revision/RESPONSE_CLAUDE.md`
- NNFT-135 (age/integer disambiguation) resolved by age removal — close as "resolved by removal"
- NNFT-117 (street_number demotion rules) — demotion rules can be removed alongside the type
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 geography.address.street_number removed from definitions_geography.yaml and generator
- [x] #2 identity.person.age removed from definitions_identity.yaml and generator
- [x] #3 representation.identifier.numeric_code added with: broad_type VARCHAR, clear description emphasising VARCHAR cast and leading-zero preservation, validation pattern, generator, samples
- [x] #4 numeric_code generator produces realistic fixed-length all-digit codes (country codes, NAICS, FIPS, product codes)
- [x] #5 Sense LabelCategoryMap updated for removed and added labels
- [x] #6 Street_number demotion rules in column.rs cleaned up (no longer needed)
- [x] #7 finetype check passes — all generators aligned with definitions
- [x] #8 NNFT-135 closed as resolved by removal
- [x] #9 Model retrained on updated 163-type taxonomy
- [x] #10 Profile eval and actionability eval baselines updated
- [x] #11 CLAUDE.md updated: taxonomy count, domain counts, decided items if needed
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Taxonomy YAML changes (remove street_number, age; add numeric_code)
2. Generator changes (remove 2, add 1)
3. LabelCategoryMap updates (remove 2, add 1)
4. Column disambiguation cleanup (column.rs rules + tests)
5. Eval infrastructure (manifest.csv + schema_mapping.yaml)
6. Model artifacts (tier_graph.json + label_index.json)
7. Verify taxonomy alignment (cargo run -- check + cargo test)
8. Retrain models
9. Update eval baselines
10. Close NNFT-135
11. Version bump to 0.5.2
12. Update CLAUDE.md, commit, tag
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Step 1-7 complete:
- Removed street_number from geography YAML, generator, LabelCategoryMap, column.rs demotion rules, eval manifest/mapping, tier_graph, label_index
- Removed age from identity YAML, generator, LabelCategoryMap, column.rs header hints/measurement disambiguation, DuckDB type mapping, Sense training data, Model2Vec prep, eval manifest/mapping, tier_graph, label_index
- Added numeric_code to representation YAML with VARCHAR broad_type, generator (ISO country codes, NAICS, FIPS, product codes), LabelCategoryMap FORMAT_LABELS, DuckDB type mapping, tier_graph VARCHAR_identifier, label_index
- Fixed pre-existing bug: model2vec_prep test used wrong tensor name \"embeddings\" vs \"type_embeddings\"
- Regenerated Model2Vec type_embeddings.safetensors (489 rows = 163 × 3)
- All tests pass: 98 (core) + 258 (model) + 40 (train) + 7 (build-tools) = 403 tests
- Taxonomy check: 163/163 generators aligned, 100% samples pass

Step 8-12 complete:
- CharCNN v10 trained: 163 classes, 161k samples (priority>=1), 5 epochs, seed 42, 83.6% accuracy
- Default symlink updated to char-cnn-v10
- Profile eval: 110/116 (94.8%) — regressed from 117/119 due to retrain boundary shifts
- Actionability: 98.7% (improved from 96.0%) — long_full_month_date resolved
- NNFT-135 closed as resolved by age removal
- Version bumped to 0.5.2
- CLAUDE.md updated with new taxonomy counts and milestone
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Taxonomy revision: removed 2 problematic types, added 1 new type, retrained model.

**Removals:**
- `geography.address.street_number` — validation pattern matched plain integers, causing false positives. Demotion rules in column.rs (NUMERIC_ATTRACTORS, disambiguate_numeric detection block) removed alongside the type.
- `identity.person.age` — CAST(col AS SMALLINT) identical to integer_number. 205 SOTAB false positives at 0.995 confidence. Resolves NNFT-135.

**Addition:**
- `representation.identifier.numeric_code` — All-digit VARCHAR codes with leading zeros (ISO country numeric 840/036, NAICS, FIPS, product codes). broad_type: VARCHAR, transform: CAST({col} AS VARCHAR). Generator produces 4 realistic code patterns. Addresses #2 analyst frustration: leading zeros stripped when auto-inferred as integers.

**Net effect:** 164 → 163 types across 7 domains (geography 16→15, identity 20→19, representation 31→32).

**Files changed (25+):**
- Taxonomy: definitions_geography.yaml, definitions_identity.yaml, definitions_representation.yaml
- Core: generator.rs (removed 2 generators, added 1)
- Model: label_category_map.rs (category arrays + tests), column.rs (demotion rules, header hints, MEASUREMENT_TYPES, 6 tests updated/removed), semantic.rs, type_mapping.rs
- Train: data.rs, model2vec_prep.rs (also fixed pre-existing tensor name bug)
- Eval: manifest.csv, schema_mapping.yaml, schema_mapping.csv
- Model artifacts: tier_graph.json, label_index.json, type_embeddings.safetensors (regenerated)
- Infrastructure: Cargo.toml (version 0.5.2), CLAUDE.md

**Model retrain:**
- CharCNN v10: 163 classes, 161,000 samples, 5 epochs, 83.6% training accuracy
- Model2Vec type embeddings regenerated (489 rows = 163 × 3 FPS representatives)
- Default symlink updated: models/default → char-cnn-v10

**Eval baselines:**
- Profile eval: 110/116 (94.8%) — regressed from 117/119 due to retrain. 4 new misclassifications (utc_offset, ean, name disambiguation). Follow-up needed.
- Actionability: 98.7% (2990/3030) — improved from 96.0%. long_full_month_date now correctly classified.

**Tests:** 363 pass (98 core + 258 model + 7 build-tools), taxonomy check 163/163 aligned."
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
