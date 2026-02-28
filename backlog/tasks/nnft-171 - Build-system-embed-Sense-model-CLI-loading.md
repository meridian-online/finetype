---
id: NNFT-171
title: 'Build system: embed Sense model + CLI loading'
status: To Do
assignee: []
created_date: '2026-02-28 23:07'
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
- [ ] #1 models/sense/ directory with copied spike artifacts
- [ ] #2 build.rs generates HAS_SENSE_CLASSIFIER, SENSE_MODEL, SENSE_CONFIG constants
- [ ] #3 CLI loading sequence: shared Model2VecResources → Semantic + Entity + Sense → ColumnClassifier
- [ ] #4 finetype --no-sense flag forces legacy pipeline
- [ ] #5 cargo build succeeds with embedded models
- [ ] #6 make ci passes
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
