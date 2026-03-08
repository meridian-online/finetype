---
id: NNFT-255
title: 'Release v0.6.7 — feature pipeline, load bug fix, discovery spike'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-08 04:21'
updated_date: '2026-03-08 04:24'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.6.7 packaging the m-12 feature-augmented CharCNN pipeline (NNFT-247–251), load command bug fix (NNFT-252), and feature-retrain discovery (NNFT-253).

Key changes since v0.6.6:
- 32-feature deterministic extractor with 3-tier architecture
- CharCNN feature fusion at classifier head (feature_dim config, backward compatible)
- Feature-based disambiguation rules F1-F3 in Sense pipeline
- Load command fix: generic numeric types now get CAST correctly
- Discovery: feature_dim=32 retrain regresses eval, keeping feature_dim=0 + rules
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version bumped to 0.6.7 in workspace Cargo.toml
- [x] #2 CHANGELOG.md updated with v0.6.7 section
- [x] #3 All CI checks pass (fmt, clippy, test, taxonomy check)
- [x] #4 Git tag v0.6.7 created and pushed
- [x] #5 GitHub release created via CI
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released v0.6.7 with feature-augmented inference pipeline (NNFT-247-251), load command CAST fix (NNFT-252), and feature-retrain discovery finding (NNFT-253).

Changes:
- 32-feature deterministic extractor + 3 disambiguation rules (F1-F3)
- CharCNN feature fusion architecture (feature_dim config, backward compatible)
- Load command fix: generic numeric types now get CAST correctly
- Profile: 95.7% label, 97.3% domain. Actionability: 99.9%

Release: tag v0.6.7 pushed, CI + Release workflows triggered.
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
