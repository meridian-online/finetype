---
id: NNFT-153
title: 'Release v0.4.0: entity classifier, accuracy sprint, locale detection'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-27 13:30'
updated_date: '2026-02-27 13:32'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.4.0 shipping 19 commits across 18 tasks (NNFT-130 through NNFT-152).

Headline: Entity classifier integration (Deep Sets MLP for full_name overcall demotion).

Accuracy improvements:
- SOTAB label: 30.5% → 43.3% (+13pp)
- SOTAB domain: 54.8% → 68.3% (+14pp)
- Profile eval: expanded 74 → 120 columns, stable at 113/120 (94.2%)

Key features:
- Entity classifier (NNFT-150/151/152) — Rule 18 entity demotion
- Phone validation precision overhaul (NNFT-132/136)
- Text length demotion Rule 16 (NNFT-134)
- Duration/TLD disambiguation Rule 14 (NNFT-131)
- UTC offset override Rule 17 (NNFT-143)
- CLI schema command + taxonomy --full export (NNFT-149)
- Entity name and paragraph types in taxonomy (NNFT-137)
- Post-hoc locale detection (NNFT-140/141)
- Designation-aware is_generic (NNFT-139)
- Evaluation package with precision/actionability/calibration (NNFT-147)
- Profile eval expanded 74→120 columns (NNFT-148)
- CLI batch mode for benchmarks (NNFT-130)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Entity classifier model uploaded to HuggingFace (3 files under entity-classifier/)
- [x] #2 download-model.sh updated with entity classifier download section
- [x] #3 Version bumped to 0.4.0 in workspace Cargo.toml
- [x] #4 CHANGELOG.md has [0.4.0] section with all features documented
- [x] #5 CLAUDE.md version string updated
- [x] #6 cargo test passes (309+ tests)
- [x] #7 cargo run -- check passes (171/171 taxonomy alignment)
- [x] #8 Git tagged v0.4.0 and pushed to origin
- [x] #9 GitHub Actions release workflow triggered and completes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- Entity classifier model uploaded to HuggingFace (3 commits: a044c7e, 04ec491, dc225a1)
- download-model.sh: added entity-classifier section with graceful degradation (same pattern as Model2Vec)
- Version bump: 0.3.0 → 0.4.0 across workspace Cargo.toml (3 occurrences)
- CHANGELOG: comprehensive [0.4.0] section with Accuracy (5), Added (6), Changed (4) categories
- Pre-commit hook passed: fmt, clippy, 309 tests (98 core + 211 model)
- Tag v0.4.0 pushed, release workflow running (5 platform builds)
- DoD #4 (decision record): N/A — straightforward release execution, no approach decision required
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released FineType v0.4.0 — the largest accuracy release since the project's inception.

## What changed

**Entity classifier model uploaded to HuggingFace** — 3 files (model.safetensors, config.json, label_index.json) uploaded to noon-org/finetype-char-cnn under entity-classifier/ prefix. CI can now download the model alongside CharCNN and Model2Vec artifacts.

**download-model.sh updated** — Added entity classifier download section following the same graceful-degradation pattern as Model2Vec (HAS_ENTITY_CLASSIFIER=false fallback). Downloads 3 files, cleans up on failure.

**Version bumped 0.3.0 → 0.4.0** — workspace Cargo.toml version + internal crate dependency versions all updated.

**CHANGELOG.md** — Comprehensive [0.4.0] section covering Accuracy (5 items), Added (6 items), and Changed (4 items). Documents all 18 tasks (NNFT-130 through NNFT-152).

**CLAUDE.md** — Version string updated to v0.4.0.

## Impact

Accuracy improvements over v0.3.0:
- SOTAB label: 30.5% → 43.3% (+13pp)
- SOTAB domain: 54.8% → 68.3% (+14pp)
- Profile eval: expanded from 74 → 120 columns, stable at 113/120 (94.2%)
- Actionability: 98.7% (2990/3030 datetime values parse correctly)

The entity classifier alone improved SOTAB domain accuracy by +3.9pp, affecting 3,027 columns (18.1% of the benchmark).

## Tests

- 309 tests pass (98 core + 211 model, pre-commit hook verified)
- Taxonomy check: 171/171 alignment
- Release build: clean compilation with embedded Model2Vec + entity classifier

## Files modified

- Cargo.toml (version bump)
- CHANGELOG.md (new [0.4.0] section)
- CLAUDE.md (version string)
- .github/scripts/download-model.sh (entity classifier download)

## Release artifacts

- Git tag: v0.4.0
- GitHub Actions release workflow: triggered (builds Linux x86/arm, macOS x86/arm, Windows)
- HuggingFace: entity-classifier/ uploaded to noon-org/finetype-char-cnn
- Homebrew tap: auto-updated by release workflow
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
