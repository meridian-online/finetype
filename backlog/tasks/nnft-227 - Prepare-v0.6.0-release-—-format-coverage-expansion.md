---
id: NNFT-227
title: Prepare v0.6.0 release — format coverage expansion
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-05 08:47'
updated_date: '2026-03-05 08:47'
labels:
  - release-prep
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.6.0 with 53 new format types (163→216), CharCNN-v12 model, and pipeline improvements. This is the Format Coverage milestone release.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Version bumped to 0.6.0 in all Cargo.toml files
- [ ] #2 CHANGELOG.md updated with v0.6.0 entry
- [ ] #3 char-cnn-v12 model uploaded to HuggingFace (hughcameron/finetype)
- [ ] #4 CI passes (fmt, clippy, test, taxonomy check)
- [ ] #5 GitHub release created via tag push
- [ ] #6 Homebrew tap updated automatically by CI
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Version bump: update all Cargo.toml files to 0.6.0
2. CHANGELOG: write v0.6.0 entry with all changes since v0.5.3
3. Upload char-cnn-v12 model to HuggingFace
4. Run CI checks locally (fmt, clippy, test, check)
5. Commit, tag v0.6.0, push
6. Verify CI release workflow and Homebrew tap update
<!-- SECTION:PLAN:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
