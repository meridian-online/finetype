---
id: NNFT-146
title: Snapshot Learning — prevent model loss during retraining
status: To Do
assignee:
  - '@nightingale'
created_date: '2026-02-27 01:02'
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
- [ ] #1 Running `finetype train --output models/X` automatically snapshots the existing models/X directory before overwriting (with a clear log message showing the snapshot path)
- [ ] #2 Training command accepts a `--seed` flag for deterministic reproducibility
- [ ] #3 Each trained model directory contains a manifest with training metadata: data file, epochs, seed, timestamp, and parent model (if any)
- [ ] #4 CLAUDE.md Decided Items documents the Snapshot Learning practice
- [ ] #5 Snapshot mechanism is tested — overwriting a model dir produces a recoverable backup
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
