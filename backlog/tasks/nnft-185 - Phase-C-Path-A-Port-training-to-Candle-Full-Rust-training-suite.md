---
id: NNFT-185
title: 'Phase C (Path A): Port training to Candle - Full Rust training suite'
status: To Do
assignee: []
created_date: '2026-03-02 07:23'
labels:
  - phase-c-path-a
  - training
  - candle
  - depends-on-phase-0
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**BLOCKED until Phase 0 spike completes with "Yes" or "Partial" recommendation.**

Port all PyTorch training scripts to Candle Rust. Only proceed if Phase 0 spike validates Candle viability.

**Objective**: Replace 8 Python training scripts with Rust training binaries using Candle.

**Work** (only if Phase 0 succeeds):
1. Create `crates/finetype-train/` crate with binaries:
   - `train-sense-model` — Sense classifier training (cross-attention, Model2Vec integration)
   - `train-entity-classifier` — Entity MLP training
   - `prepare-sense-data` — Data preparation (SOTAB + profile eval + synthetic headers)

2. Delete Python training scripts:
   - `scripts/train_sense_model.py`
   - `scripts/train_entity_classifier.py`
   - `scripts/prepare_sense_data.py`
   - `scripts/prepare_model2vec.py`

3. Update Makefile with Rust training targets
4. Document pure Rust training workflow

**Acceptance criteria** (Path A only):
- `cargo run --release --bin train-sense-model -- --output models/sense_prod` trains successfully
- Final model achieves ≥parity accuracy to PyTorch baseline (116/120 on profile eval)
- `cargo run --release --bin train-entity-classifier` trains and serializes correctly
- All models serialize to safetensors with perfect round-trip fidelity
- No Python venv required for training
- CI/training pipeline requires no Python

**Note**: Do NOT start until Phase 0 spike completes with Path A recommendation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Phase 0 spike completed with Path A (Candle viable) recommendation
- [ ] #2 Create finetype-train crate with Candle dependencies
- [ ] #3 Implement train-sense-model binary with cross-attention and Model2Vec
- [ ] #4 Implement train-entity-classifier binary with Deep Sets MLP
- [ ] #5 Implement prepare-sense-data data pipeline
- [ ] #6 Run training with >90% accuracy parity to PyTorch baseline
- [ ] #7 Verify safetensors serialization with round-trip testing
- [ ] #8 Delete Python training scripts (train_sense, train_entity, prepare_sense, prepare_model2vec)
- [ ] #9 Update Makefile with Rust training targets
- [ ] #10 Update DEVELOPMENT.md documenting pure Rust training workflow
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
