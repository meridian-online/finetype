---
id: NNFT-236
title: Release v0.6.3 — taxonomy cleanup + accuracy recovery
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-06 20:32'
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
- [ ] #1 Version bumped to 0.6.3 in Cargo.toml and Cargo.lock
- [ ] #2 CHANGELOG.md updated with v0.6.3 entry
- [ ] #3 cargo test + cargo run -- check pass
- [ ] #4 char-cnn-v13 model uploaded to HuggingFace
- [ ] #5 GitHub release created via tag push
- [ ] #6 Homebrew tap updated (automated via CI)
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
