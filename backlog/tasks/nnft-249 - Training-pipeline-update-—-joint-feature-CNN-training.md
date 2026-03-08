---
id: NNFT-249
title: Training pipeline update — joint feature + CNN training
status: To Do
assignee: []
created_date: '2026-03-07 23:55'
labels:
  - model
  - training
milestone: m-12
dependencies:
  - NNFT-247
  - NNFT-248
references:
  - crates/finetype-train/src/training.rs
  - crates/finetype-train/src/data.rs
  - scripts/train.sh
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Update the training pipeline to generate feature vectors during data preparation and train the augmented CharCNN end-to-end. The training loop must:

1. Run feature extraction on each training sample alongside character encoding
2. Pass both character sequences and feature vectors through the augmented model
3. Backpropagate through both CNN and fusion layers jointly
4. Support the existing training infrastructure (snapshots, seed determinism, Metal/CUDA auto-detection)

Explore hyperparameter space: wider filters, more layers, higher samples_per_type — within 30-hour training budget on Metal/CUDA.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Training data pipeline extracts features for each sample alongside character encoding
- [ ] #2 Training loop passes both inputs through augmented model
- [ ] #3 Loss backpropagates through both CNN conv layers and fusion layers
- [ ] #4 Snapshot learning preserved — auto-snapshot before overwriting
- [ ] #5 Deterministic training with --seed N still works
- [ ] #6 Metal/CUDA auto-detection unchanged
- [ ] #7 Training completes within 30-hour budget for full 250-type taxonomy
- [ ] #8 Training script (scripts/train.sh) updated to support feature-augmented mode
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
