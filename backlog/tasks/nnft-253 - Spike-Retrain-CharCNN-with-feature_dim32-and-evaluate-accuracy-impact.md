---
id: NNFT-253
title: 'Spike: Retrain CharCNN with feature_dim=32 and evaluate accuracy impact'
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-08 01:36'
updated_date: '2026-03-08 01:52'
labels:
  - discovery
  - model
  - m-12
dependencies: []
references:
  - crates/finetype-model/src/features.rs
  - crates/finetype-model/src/charcnn.rs
  - crates/finetype-model/src/column.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Question:** Does training CharCNN with the 32 deterministic features fused at fc1 improve accuracy over the current feature_dim=0 model + post-vote disambiguation rules?

**Context:** m-12 (NNFT-247–250) added feature extraction and parallel fusion architecture, but the current char-cnn-v14-250 model trains with feature_dim=0. Features are only used post-vote via hand-written rules F1–F3. This spike trains a model that actually uses the features during forward pass and measures the accuracy delta.

**Time budget:** ~2 hours (train + eval + write-up)

**Approach:**
1. Train char-cnn-v15-250 with feature_dim=32, same data/epochs as v14
2. Run full eval suite (profile + actionability)
3. Compare with/without post-vote rules F1–F3
4. Document findings: does the model learn what the rules do? Any regressions?

**Success = knowledge:** A written finding with accuracy numbers, not necessarily a new default model.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 char-cnn-v15-250 trained with feature_dim=32 on same data as v14 (1500 samples/type, 10 epochs)
- [ ] #2 Full eval suite run: profile eval + actionability
- [ ] #3 Side-by-side comparison table: v14+rules vs v15 vs v15+rules
- [ ] #4 Written finding documenting accuracy delta, regressions, and recommendation
- [ ] #5 If v15 is better: recommend as new default. If not: document why and close.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Generate fresh training data at 1500 samples/type (v14 used training-v14-250.ndjson, 372k samples)
2. Train char-cnn-v15-250 with --use-features (feature_dim=32), same arch (small: 32/64/128), 10 epochs, seed 42
3. Update models/default symlink to v15, run full eval suite (profile + actionability)
4. Record v15 results, then compare with v14 baseline (already known: 178/186 profile, 99.9% actionability)
5. Test v15 with F1-F3 rules disabled to see if model learned those patterns
6. Write finding in discovery/feature-retrain/FINDING.md
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Training data generated: 372k samples (1500/type × 250 types) at training.ndjson.
Beelink CPU too slow (~8h for 10 epochs). Training moved to M1 Mac with Metal.
Partial v15 model dir cleaned up.
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
