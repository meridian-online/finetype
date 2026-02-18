---
id: NNFT-087
title: Release v0.1.7 with tiered model as default
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-17 22:44'
updated_date: '2026-02-17 23:06'
labels:
  - release
  - model
dependencies:
  - NNFT-084
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Ship v0.1.7 with the tiered model architecture as the default inference mode. This release includes the ValueClassifier trait, SI number disambiguation (Rule 9), --model-type CLI flag, and the tiered-v2 model trained with 30 epochs.

Key changes since v0.1.6:
- ValueClassifier trait enabling polymorphic classifier selection
- SI number disambiguation rule in column.rs
- --model-type tiered/flat CLI flag on infer and profile commands
- tiered-v2 model (72.6% format-detectable label accuracy, +4.5pp over flat baseline)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version bumped to 0.1.7 in Cargo.toml and CHANGELOG updated
- [x] #2 tiered-v2 uploaded to HuggingFace and download-model.sh updated
- [x] #3 Default model switched from char-cnn-v5 (flat) to tiered-v2
- [x] #4 Release binary embeds tiered-v2 model and uses tiered inference by default
- [x] #5 GitHub release created with CI-built binaries for all platforms
- [x] #6 Homebrew formula updated to v0.1.7
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add TieredClassifier::from_bytes() method to finetype-model (needs tier_graph bytes + array of (name, weights, labels, config) tuples)
2. Update build.rs to detect tiered model (via tier_graph.json) and embed all tier subdirectories
3. Update main.rs: change default model_type from char-cnn to tiered, add embedded tiered model loading
4. Update download-model.sh to handle tiered model (download all tier subdirs + tier_graph.json)
5. Update models/default symlink: char-cnn-v7 → tiered-v2
6. Bump version 0.1.6 → 0.1.7 in workspace Cargo.toml
7. Upload tiered-v2 to HuggingFace noon-org/finetype-char-cnn repo
8. Run all tests, verify embedded build works
9. Commit, push, tag v0.1.7 → CI builds + Homebrew auto-update"
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1-4 complete:
- Version bumped to 0.1.7 in workspace Cargo.toml
- tiered-v2 uploaded to HuggingFace (42de626)
- models/default symlink updated: char-cnn-v7 → tiered-v2
- build.rs auto-detects tiered model, embeds all 34 tier subdirs + tier_graph.json
- CLI defaults to --model-type tiered on infer, train, eval, profile commands
- download-model.sh supports tiered via manifest.txt
- All 187 tests pass, clippy + fmt clean
- Binary size: 21MB (was 12MB with flat model)

Remaining: AC#5 (GitHub release) and AC#6 (Homebrew) — need to commit, push, tag

Committed b328379, pushed to main, tagged v0.1.7. Release CI running (run 22119089208). Waiting for builds + Homebrew auto-update.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released v0.1.7 with tiered model as default inference engine.

## Changes
- **Default model**: Switched from flat CharCNN v7 to tiered-v2 (72.6% format-detectable label accuracy, +4.5pp over flat baseline)
- **build.rs**: Auto-detects tiered model via tier_graph.json, generates embedded lookup function for all 34 tier subdirectories
- **CLI**: Defaults to `--model-type tiered` on infer, train, eval, profile commands
- **download-model.sh**: Supports tiered models via manifest.txt file listing
- **Binary size**: ~21MB (up from ~12MB due to 34 embedded models)

## Infrastructure
- tiered-v2 uploaded to HuggingFace noon-org/finetype-char-cnn (commit 42de626)
- GitHub release created with binaries for all 4 platforms
- Homebrew formula auto-updated to v0.1.7
- All 187 tests pass, clippy + fmt clean

## Commits
- b328379: v0.1.7 release commit (build.rs, main.rs, download-model.sh, version bump, backlog)
- v0.1.7 tag → CI release run 22119089208 (all 6 jobs succeeded)"
<!-- SECTION:FINAL_SUMMARY:END -->
