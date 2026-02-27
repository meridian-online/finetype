---
id: NNFT-146
title: Snapshot Learning — prevent model loss during retraining
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-27 01:02'
updated_date: '2026-02-27 21:04'
labels:
  - quality
  - training
  - infrastructure
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
We lost the NNFT-137 retrained model (69/74, 70/74 with Rule 17) because we overwrote it during a city generator expansion retrain. Training is non-deterministic (no seed support), so the model was irrecoverable — three retrain attempts with the same data produced 67/74, 66/74, and varying results. We eventually had to restore v0.3.0 models from HuggingFace (169 types, pre-entity_name).

This task establishes a quality practice: never overwrite a model without snapshotting it first. The practice should be lightweight, automatic where possible, and hard to skip.

Scope includes:
- Pre-training model snapshot mechanism (copy current models to timestamped backup before any `finetype train` writes)
- Seed support in the training command (deterministic training when needed)
- Model provenance metadata (what data, what params, what score produced this model)
- Documentation of the practice in CLAUDE.md
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Running `finetype train --output models/X` automatically snapshots the existing models/X directory before overwriting (with a clear log message showing the snapshot path)
- [x] #2 Training command accepts a `--seed` flag for deterministic reproducibility
- [x] #3 Each trained model directory contains a manifest with training metadata: data file, epochs, seed, timestamp, and parent model (if any)
- [x] #4 CLAUDE.md Decided Items documents the Snapshot Learning practice
- [x] #5 Snapshot mechanism is tested — overwriting a model dir produces a recoverable backup
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add `seed: Option<u64>` to CharTrainingConfig and TieredTrainingConfig
   - When Some, use StdRng::seed_from_u64 instead of thread_rng
   - When None, preserve current non-deterministic behaviour

2. Add `--seed` flag to CLI Train command
   - Thread it through to the config structs

3. Add auto-snapshot to cmd_train()
   - Before writing to output dir, if it exists and contains model files, copy it to `{output}.snapshot.{timestamp}`
   - Log the snapshot path clearly to stderr
   - Lightweight: just a directory copy, no new dependencies

4. Add training manifest (manifest.json) to saved model output
   - Written after training completes, alongside model.safetensors
   - Fields: data_file, epochs, batch_size, seed, timestamp, model_type, n_classes, n_samples, parent_snapshot (path to snapshot if one was taken)
   - CharTrainer and TieredTrainer both write it; tiered also writes per-node accuracy

5. Add test for snapshot mechanism
   - Create a temp dir with dummy model files, run snapshot logic, verify backup exists and original is intact

6. Update CLAUDE.md Decided Items with Snapshot Learning practice

7. Commit with NNFT-146 reference
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented Snapshot Learning — a three-part training safety practice that prevents model loss during retraining.

Changes:
- Added `seed: Option<u64>` to `CharTrainingConfig` and `TieredTrainingConfig` — when set, uses `StdRng::seed_from_u64` instead of `thread_rng()` for deterministic shuffle order across epochs
- Added `--seed` CLI flag to `finetype train` command, threaded through to both CharCNN and Tiered training configs
- Added auto-snapshot in `cmd_train()` — before writing to an output directory that already contains model artifacts (model.safetensors, tier_graph.json, or tier0/model.safetensors), copies the entire directory tree to `{name}.snapshot.{ISO-timestamp}` with stderr logging
- Added `TrainingManifest` struct that writes `manifest.json` alongside model artifacts — records data_file, epochs, batch_size, seed, timestamp, model_type, n_classes, n_samples, and parent_snapshot path
- Added `chrono` (workspace) and `tempfile` (dev) dependencies to finetype-cli
- CLAUDE.md Decided Item 19 documents the practice and its motivation

Files changed:
- crates/finetype-model/src/char_training.rs — seed field + seeded RNG
- crates/finetype-model/src/tiered_training.rs — seed field + seeded RNG
- crates/finetype-cli/src/main.rs — --seed flag, snapshot_model_dir(), copy_dir_recursive(), TrainingManifest, 7 tests
- crates/finetype-cli/Cargo.toml — chrono + tempfile deps
- Cargo.toml — tempfile workspace dep
- CLAUDE.md — Decided Item 19, updated train command docs

Tests: 316 pass (7 new snapshot/manifest tests + 98 core + 211 model), clippy clean, fmt clean, taxonomy check passes.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
