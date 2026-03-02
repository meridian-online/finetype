---
id: NNFT-185
title: 'Phase C (Path A): Port training to Candle - Full Rust training suite'
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-02 07:23'
updated_date: '2026-03-02 10:14'
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
- [x] #1 Phase 0 spike completed with Path A (Candle viable) recommendation
- [x] #2 Create finetype-train crate with Candle dependencies
- [ ] #3 Implement train-sense-model binary with cross-attention and Model2Vec
- [ ] #4 Implement train-entity-classifier binary with Deep Sets MLP
- [ ] #5 Implement prepare-sense-data data pipeline
- [ ] #6 Run training with >90% accuracy parity to PyTorch baseline
- [ ] #7 Verify safetensors serialization with round-trip testing
- [ ] #8 Delete Python training scripts (train_sense, train_entity, prepare_sense, prepare_model2vec)
- [ ] #9 Update Makefile with Rust training targets
- [ ] #10 Update DEVELOPMENT.md documenting pure Rust training workflow
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Phase C implementation plan — Training-first with fixtures approach

## Step 1: Scaffold finetype-train crate
- Cargo.toml with candle-core/nn 0.8, half 2.4, duckdb, arrow, parquet, clap, safetensors
- Depends on finetype-core (taxonomy) + finetype-model (Model2VecResources, entity features)
- 4 binary targets: train-sense-model, train-entity-classifier, prepare-sense-data, prepare-model2vec
- Move spike model code (SenseModelA, EntityClassifier) into clean modules

## Step 2: Training infrastructure (shared)
- EarlyStopping struct (patience, best metric tracking)
- CosineScheduler (learning rate annealing)
- Cross-entropy loss (log_softmax + gather, validated in spike)
- TrainingMetrics (per-epoch recording, JSON serialisable)
- Deterministic seeding

## Step 3: Sense training loop with fixture data
- Create small fixture dataset (50 columns, known broad categories + entity subtypes)
- Real training loop: batching, forward, loss, backward, optimizer step
- AdamW + cosine annealing (matching Python hyperparams)
- Header dropout 50% during training
- Dual-head loss: weighted CE broad + CE entity (entity-only columns)
- Early stopping on validation broad accuracy
- Safetensors save with config.json manifest
- Verify: loss decreases, accuracy > 80% on fixture

## Step 4: Entity classifier training loop
- Reuse compute_features() / compute_stat_features() from finetype-model::entity
- Training loop with class-weighted CE, AdamW, cosine annealing
- 5-fold stratified CV on training data
- Demotion analysis at configurable threshold
- Safetensors save with config.json + label_index.json

## Step 5: Data pipeline (prepare-sense-data)
- Load SOTAB parquet via DuckDB
- SOTAB Schema.org → 6 broad categories + 4 entity subtypes (static HashMap)
- Value sampling: top-K by frequency + random fill (max 50)
- Profile eval column loading from manifest CSV
- Synthetic header generation (curated templates per label)
- Model2Vec encoding via Model2VecResources
- JSONL output with pre-computed embeddings
- Stratified train/val split (80/20)

## Step 6: Model2Vec type embeddings (prepare-model2vec)
- Load taxonomy via finetype-core::Taxonomy
- Synonym expansion (title + aliases + label components)
- Encode synonyms with Model2VecResources::encode_batch()
- FPS algorithm (K=3 representatives per type)
- Write type_embeddings.safetensors + label_index.json

## Step 7: Full training run + accuracy validation
- Prepare data from SOTAB + profile (v0.5.1 taxonomy)
- Train Sense model on real data
- Train Entity classifier on real data
- Run profile_eval.sh — target: >=116/120
- Verify safetensors loadable by finetype-model::SenseClassifier

## Step 8: Cleanup & integration
- Update Makefile with Rust training targets
- Delete Python scripts: train_sense_model.py, train_entity_classifier.py, prepare_sense_data.py, prepare_model2vec.py
- Update CLAUDE.md
- Run full CI (fmt, clippy, test, check)
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Step 1-2 complete: finetype-train crate scaffolded with shared training infrastructure. 10 unit tests pass. Commit 10b0735.
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
