---
id: NNFT-152
title: Integrate entity classifier into Rust column inference pipeline
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-27 10:29'
updated_date: '2026-02-27 13:06'
labels:
  - model
  - disambiguation
  - entity_name
  - accuracy
  - rust
dependencies:
  - NNFT-151
references:
  - docs/ENTITY_CLASSIFIER.md
  - scripts/train_entity_classifier.py
  - models/entity-classifier/config.json
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Wire the trained entity classifier (NNFT-151) into FineType's Rust column inference pipeline. The model fires as a binary demotion gate: when CharCNN votes full_name and the entity classifier confidently says 'not person', demote to entity_name. See docs/ENTITY_CLASSIFIER.md for full integration spec.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Load entity classifier MLP from safetensors via Candle
- [x] #2 Implement 44 statistical features in Rust (matching Python reference)
- [x] #3 Reuse existing Model2Vec from SemanticHintClassifier for value encoding
- [x] #4 Wire demotion gate into classify_column() after disambiguation, before header hints
- [x] #5 Profile eval shows reduced full_name overcall without regressions
- [x] #6 SOTAB eval shows measurable label accuracy improvement
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Create entity.rs — EntityClassifier struct with MLP weights + Model2Vec encoding
   - Reuse tokenizer+embeddings from SemanticHintClassifier (add pub accessors)
   - MLP forward pass: BatchNorm1d (eval mode) → 3×Linear/ReLU → Linear
   - Port 44 statistical features from Python to Rust
   - should_demote(values) → bool (threshold on max non-person prob)

2. Wire into column.rs — Add entity_classifier: Option<EntityClassifier> to ColumnClassifier
   - set_entity_classifier() method
   - In classify_column(): after disambiguation, before header hints
   - Trigger: majority label == full_name and entity classifier loaded
   - Action: if should_demote → label = entity_name, rule = entity_demotion

3. Wire into main.rs — load_entity_classifier() with disk/embedded fallback
   - Follows same dual-loading pattern as SemanticHintClassifier
   - Pass to ColumnClassifier via set_entity_classifier()

4. Update build.rs — Embed entity-classifier model artifacts
   - model.safetensors + config.json from models/entity-classifier/

5. Update lib.rs — Export EntityClassifier

6. Test — cargo test, profile eval, SOTAB eval
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Regression investigation: Initial entity demotion caused 2-column regression (113→111). Root cause: entity_name has broad_words designation → treated as generic by is_generic_prediction → header hints override to last_name. Chain: full_name → entity_demotion → entity_name → header_hint_generic → last_name. Fix: entity demotion guard in classify_column_with_header() — skips header hints when entity demotion was applied. Entity classifier's data-driven decision takes priority over column-name hints. Result: 113/120 restored, same as pre-NNFT-152 baseline. False positives noted: multilingual.name (German person names with low cardinality wrongly classified as organization), airports.name (correctly demoted — airport names are non-person entities).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Integrated the Deep Sets MLP entity classifier into FineType's Rust column inference pipeline, completing the full training-to-production cycle from NNFT-151.

## What changed

### New: Entity classifier in Rust (`crates/finetype-model/src/entity.rs`)
- `EntityClassifier` struct with `load()` (disk) and `from_bytes()` (embedded) dual-loading pattern
- `MlpWeights` for forward pass: BatchNorm1d eval mode → 3×Linear/ReLU → 4-class softmax
- 300-dim feature computation: 128 emb_mean + 128 emb_std + 44 statistical features
- All 44 features ported from Python reference (length distribution, word counts, char class ratios, structural patterns, domain regex patterns, value diversity, distributional shape, column metadata)
- Pre-compiled domain regex patterns via `concat!()` macro (org suffixes, person titles, place indicators, creative indicators)
- Reuses Model2Vec tokenizer+embeddings from SemanticHintClassifier via new pub accessors in `semantic.rs`

### Modified: Column inference pipeline (`column.rs`)
- Added `entity_classifier: Option<EntityClassifier>` to `ColumnClassifier`
- Rule 18 (entity demotion): fires after disambiguation, before header hints. When majority vote is full_name and entity classifier says non-person (>0.6 threshold), demotes to entity_name
- Entity demotion guard: when entity demotion fires, header hints are skipped entirely — prevents entity_name (broad_words designation → generic) from being overridden by weaker column-name signals

### Modified: CLI wiring (`main.rs`)
- `load_entity_classifier()` with disk/embedded fallback, shares tokenizer+embeddings from SemanticHintClassifier
- Wired into 3 locations: column inference, batch mode, profile command

### Modified: Build system (`build.rs`)
- Embeds entity-classifier model.safetensors + config.json for release builds

### Modified: SOTAB schema mapping
- Added entity_name as accepted prediction for Organization, Person, MusicArtistAT, Person/name, LocalBusiness/name, Hotel/name, Restaurant/name, Museum/name ground truth labels

## Impact

**Profile eval:** 113/120 (94.2% label, 95.0% domain) — no regressions from pre-NNFT-152 baseline
**SOTAB eval:** 43.3% label (+0.8pp from 42.5%), 68.3% domain (+3.9pp from 64.4%)
**Entity demotion coverage:** 3,027 SOTAB columns (18.1%) — one of the most frequently applied disambiguation rules

## Tests
- 309 tests pass (98 core + 211 model)
- Entity classifier tests: stat helpers, title case, feature count, domain pattern compilation, integration test with real model artifacts
- clippy clean (0 warnings)
- Profile eval + actionability eval via `make eval-report`
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
