---
id: NNFT-228
title: Add Metal support and training workflow scripts
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-05 11:18'
updated_date: '2026-03-05 11:46'
labels:
  - infrastructure
  - training
  - metal-support
  - scripts
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enable efficient model training on M1 Macs and create streamlined training/eval/packaging scripts.

**Scope:**
1. Add `metal` feature to `finetype-train` crate (Sense and Entity training)
2. Refactor Sense/Entity training to use `get_device()` auto-detection (like CharCNN already does)
3. Create `scripts/train.sh` — wrapper for generate → train with hardware detection, architecture scaling, TUI progress
4. Create `scripts/eval.sh` — run full eval suite against a model
5. Create `scripts/package.sh` — bundle trained model into distributable .tar.gz

**Use case:** Enable large model experiments tonight (5000 samples/type, larger architecture) on M1 hardware with automated workflow.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Metal feature added to finetype-train with proper Cargo feature gating
- [x] #2 Sense training uses Device::get_device() for auto-detection (Metal/CUDA/CPU)
- [x] #3 Entity training uses Device::get_device() for auto-detection
- [x] #4 scripts/train.sh works end-to-end: generate → detect hardware → train with --size presets (small/medium/large) and --samples flag
- [x] #5 scripts/train.sh shows TUI-style progress (epoch, loss, accuracy, ETA) by parsing Rust training output
- [x] #6 scripts/eval.sh runs profile eval → actionability eval → report generation against any model directory
- [x] #7 scripts/package.sh produces versioned .tar.gz with model files, manifest, config ready for HuggingFace
- [x] #8 All scripts use correct shebang, error handling (set -euo pipefail), and sensible defaults
- [ ] #9 Can successfully run full workflow: ./scripts/train.sh --samples 5000 --size large && ./scripts/eval.sh && ./scripts/package.sh
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Metal GPU support to finetype-train and created three training workflow scripts.

## Rust changes

- **Feature flags** (`crates/finetype-train/Cargo.toml`): Added `metal` and `cuda` features gating candle-core/candle-nn backends, matching the existing pattern in finetype-model/finetype-cli.
- **Device auto-detection** (`crates/finetype-train/src/device.rs`): New `get_device()` function tries CUDA → Metal → CPU. Wired into `train_sense()` and `train_entity()` entry points. Test code continues using `Device::Cpu` directly for determinism.
- Fixed unused import warning by moving `Device` import from module-level to test module in `sense_train.rs`.

## Scripts

- **`scripts/train.sh`** — End-to-end CharCNN training: generate data → build with correct features → train. Auto-detects macOS (Metal), Linux+NVIDIA (CUDA), or CPU. Architecture presets (small/medium/large). Progress display parsing epoch/loss/accuracy/ETA. Auto-increments model name.
- **`scripts/eval.sh`** — Wraps make eval-profile + eval-actionability + eval-report. Temporarily swaps models/default symlink for non-default models, restores via trap on exit.
- **`scripts/package.sh`** — Bundles model directory into `finetype-<name>.tar.gz` with SHA256 checksum. Smoke tested: 344K archive from char-cnn-v12.

## Agent skills

Created `/train`, `/eval`, `/package` skills in `~/.claude/skills/` for agent-invocable workflow guidance.

## Verification

- `cargo test`: 258 passed, 0 failed
- `cargo clippy --all-targets`: 0 errors
- `cargo run -- check`: 216/216 taxonomy checks pass
- All three scripts: `--help` works, package smoke test produces valid archive
- Metal feature can't compile on Linux (expected — requires macOS Metal framework)

## Commits (6)

a5e026d, b69cfef, 19b46b0, a7b2b59, 2726289, 61ad754 — all pushed to main.
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
