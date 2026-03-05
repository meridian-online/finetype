---
id: NNFT-227
title: Prepare v0.6.0 release — format coverage expansion
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-05 08:47'
updated_date: '2026-03-05 09:30'
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
- [x] #1 Version bumped to 0.6.0 in all Cargo.toml files
- [x] #2 CHANGELOG.md updated with v0.6.0 entry
- [x] #3 char-cnn-v12 model uploaded to HuggingFace (hughcameron/finetype)
- [x] #4 CI passes (fmt, clippy, test, taxonomy check)
- [x] #5 GitHub release created via tag push
- [x] #6 Homebrew tap updated automatically by CI
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

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released FineType v0.6.0 — Format Coverage expansion milestone.

**What shipped:**
- 53 new format types (163→216, 33% taxonomy growth): CJK dates, Apache CLF, ISO 8601 ms/μs, Indian lakh/crore, Swiss apostrophe, accounting notation, and more
- CharCNN-v12 model (212k training samples, 216 classes)
- Pipeline fix: header-hint location override for Sense misrouting
- CLI output format alignment (label field, JSON enrichment)

**Metrics:**
- Profile: 111/116 (95.7% label, 98.3% domain)
- Actionability: 96.2% (2760/2870)
- All CI checks pass (fmt, clippy, test, taxonomy check)

**Release artifacts:**
- GitHub release with 5 platform builds (Linux x86/arm, macOS x86/arm, Windows)
- char-cnn-v12 model on HuggingFace (noon-org/finetype-char-cnn)
- Homebrew tap updated automatically

**Clippy fixes:** Two build.rs lint issues caught by CI Rust 1.93 (expect_fun_call, needless_borrows_for_generic_args) — fixed and retested.
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
