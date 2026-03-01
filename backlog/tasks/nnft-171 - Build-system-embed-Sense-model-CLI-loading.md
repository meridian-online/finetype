---
id: NNFT-171
title: 'Build system: embed Sense model + CLI loading'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-28 23:07'
updated_date: '2026-03-01 00:37'
labels:
  - sense-sharpen
  - build
dependencies:
  - NNFT-170
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Update CLI build.rs to embed Sense model artifacts (model.safetensors + config.json from models/sense/). Update CLI main.rs loading sequence: Model2VecResources → consumers → ColumnClassifier. Add --no-sense flag for A/B comparison.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 models/sense/ directory with copied spike artifacts
- [x] #2 build.rs generates HAS_SENSE_CLASSIFIER, SENSE_MODEL, SENSE_CONFIG constants
- [x] #3 CLI loading sequence: shared Model2VecResources → Semantic + Entity + Sense → ColumnClassifier
- [x] #4 finetype --no-sense flag forces legacy pipeline
- [x] #5 cargo build succeeds with embedded models
- [x] #6 make ci passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Copy spike artifacts to models/sense/ (done)
2. Add Sense model embedding to build.rs (done)
3. Add load_sense() function to main.rs (disk + embedded fallback)
4. Refactor loading: shared Model2VecResources → consumers → ColumnClassifier
5. Add --no-sense flag to Profile and Infer commands
6. Verify cargo build and make ci pass
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Integrated Sense classifier into CLI build system and loading sequence.

Changes:
- Created models/sense/ directory with Architecture A spike artifacts (model.safetensors 1.4MB, config.json)
- Extended build.rs to embed Sense model: HAS_SENSE_CLASSIFIER, SENSE_MODEL, SENSE_CONFIG constants (follows entity-classifier pattern)
- Added load_sense() to CLI main.rs with disk → embedded → None fallback
- Added load_model2vec_resources() for shared tokenizer + embedding loading
- Added wire_sense() helper that loads Sense + Model2VecResources + LabelCategoryMap and calls set_sense()
- Wired Sense into all 3 ColumnClassifier construction sites: cmd_infer column mode, cmd_infer_batch, cmd_profile
- Added --no-sense flag to both Infer and Profile commands for A/B comparison

Verification:
- cargo build succeeds (Sense artifacts embedded from models/sense/)
- All 252 tests pass
- make ci passes (fmt + clippy + test + check, 163/163 types)
- Smoke tested: batch mode shows "Loaded Sense classifier", email classified with sense_mask:format rule
- --no-sense flag correctly disables Sense (no load message, no disambiguation_rule)
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
