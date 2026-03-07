---
id: NNFT-236
title: Release v0.6.3 — taxonomy cleanup + accuracy recovery
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-06 20:32'
updated_date: '2026-03-06 20:53'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.6.3 with changes since v0.6.2:

- NNFT-233: Remove 7 low-precision types, recategorize color types (216→209 types)
- NNFT-234: Pre-retrain taxonomy rename — remove geographic names from types (eu_→dmy_, us_→mdy_, etc.)
- NNFT-235: Post-retrain accuracy recovery — 5 pipeline fixes for entity/geography confusion
- CharCNN v13 retrained on 209-type taxonomy
- CI fix for Rust 1.94 clippy/fmt

Headline metrics:
- Profile: 143/146 (97.9% label, 98.6% domain)
- Actionability: 99.3%
- Taxonomy: 209 types across 7 domains
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version bumped to 0.6.3 in Cargo.toml and Cargo.lock
- [x] #2 CHANGELOG.md updated with v0.6.3 entry
- [x] #3 cargo test + cargo run -- check pass
- [x] #4 char-cnn-v13 model uploaded to HuggingFace
- [x] #5 GitHub release created via tag push
- [x] #6 Homebrew tap updated (automated via CI)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released FineType v0.6.3 with taxonomy cleanup and accuracy recovery.

Changes since v0.6.2:
- NNFT-233: Removed 7 low-precision types (216→209)
- NNFT-234: Renamed 10 geographic type names to format-structural names
- NNFT-235: 5 pipeline fixes for entity/geography confusion (97.9% label accuracy)
- CharCNN v13 retrained and uploaded to HuggingFace (noon-org/finetype-char-cnn)
- CI fix for Rust 1.94 clippy/fmt

Release artifacts: GitHub release with 5 platform binaries, Homebrew tap updated.
Metrics: 143/146 label (97.9%), 144/146 domain (98.6%), 99.3% actionability.
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
