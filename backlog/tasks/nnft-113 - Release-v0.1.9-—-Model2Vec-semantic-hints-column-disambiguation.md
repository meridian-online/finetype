---
id: NNFT-113
title: 'Release v0.1.9 — Model2Vec semantic hints, column disambiguation'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-24 05:35'
updated_date: '2026-02-24 05:45'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.1.9 with the Model2Vec semantic column name classifier and column-level disambiguation improvements since v0.1.8.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version bumped to 0.1.9 in workspace Cargo.toml
- [x] #2 Model2Vec artifacts uploaded to HuggingFace (noon-org/finetype-char-cnn)
- [x] #3 download-model.sh updated to fetch Model2Vec files
- [x] #4 CLAUDE.md updated with v0.1.9 state
- [x] #5 All tests pass
- [x] #6 Tag v0.1.9 pushed — CI release workflow triggered
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1: Version bumped 0.1.8 → 0.1.9 in workspace Cargo.toml (3 occurrences: workspace.package.version, finetype-core dep, finetype-model dep).

AC#2: 4 Model2Vec artifacts uploaded to HuggingFace noon-org/finetype-char-cnn/model2vec/: model.safetensors (7.56MB), type_embeddings.safetensors (86.6KB), tokenizer.json, label_index.json.

AC#3: download-model.sh extended with Model2Vec section — downloads 4 files with graceful fallback (M2V_OK flag, cleanup on failure).

AC#4: CLAUDE.md updated — version 0.1.9, recent milestones include NNFT-110/NNFT-109, semantic classifier in architecture section, decided item 5a for Model2Vec, key file reference updated.

AC#5: All 221 tests pass (148 finetype-model + others). cargo fmt/clippy clean.

AC#6: Tag v0.1.9 pushed → commit d7ab923. Release CI triggered — 5 platform builds running (Linux x86/arm, macOS x86/arm, Windows).

Release CI completed — all 7 jobs green:
- 5 platform builds (Linux x86/arm, macOS x86/arm, Windows)
- Create Release: 10 assets (5 archives + 5 sha256)
- Update Homebrew Formula: tap auto-updated

GitHub release: https://github.com/noon-org/finetype/releases/tag/v0.1.9
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released FineType v0.1.9 with Model2Vec semantic column name classifier.

## What's New in v0.1.9

**Headline: Model2Vec semantic header hints (NNFT-110)**
- SemanticHintClassifier uses distilled potion-base-4M static embeddings (~7.5MB) to match column names to type labels via cosine similarity
- Replaces reliance on hardcoded header_hint() dictionary with learned semantic matching
- Profile eval improved from 55/74 → 68/74 format-detectable correct (+13, 0 regressions)
- Hardcoded header_hint() preserved as fallback — semantic classifier takes priority when above 0.70 threshold

**Column-level disambiguation (NNFT-109)**
- Unified finetype() function with column-level disambiguation
- Improved accuracy on ambiguous types through majority vote + rule-based disambiguation

## Release Artifacts
- 5 platform binaries: Linux x86/arm, macOS x86/arm, Windows
- Homebrew formula auto-updated
- GitHub release with SHA256 checksums
- CI infrastructure: download-model.sh fetches Model2Vec artifacts from HuggingFace with graceful fallback

## Changes Since v0.1.8
- Version: 0.1.8 → 0.1.9
- New file: crates/finetype-model/src/semantic.rs (SemanticHintClassifier)
- New files: models/model2vec/ (4 artifacts on HuggingFace)
- New file: scripts/prepare_model2vec.py (model preparation)
- Modified: column.rs (semantic hint integration), build.rs (model2vec embedding), main.rs (classifier wiring)
- Modified: download-model.sh (Model2Vec download section)
- Modified: CLAUDE.md (v0.1.9 state)
<!-- SECTION:FINAL_SUMMARY:END -->
