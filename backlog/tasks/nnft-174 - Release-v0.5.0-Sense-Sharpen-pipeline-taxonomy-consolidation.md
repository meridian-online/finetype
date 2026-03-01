---
id: NNFT-174
title: 'Release v0.5.0: Sense & Sharpen pipeline, taxonomy consolidation'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-01 09:04'
updated_date: '2026-03-01 09:07'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Cut the v0.5.0 release covering 26 commits since v0.4.0. Major theme: Sense & Sharpen pipeline (NNFT-163–173) as default CLI pipeline + taxonomy consolidation 171→163 types (NNFT-162) + snapshot learning (NNFT-146).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cargo.toml version bumped to 0.5.0 (workspace + internal deps)
- [x] #2 CHANGELOG.md has v0.5.0 section with Accuracy/Added/Changed/Fixed
- [x] #3 README.md type count references updated 169→163
- [x] #4 README.md domain counts table corrected (technology 30, identity 31, representation 29)
- [x] #5 README.md test count updated 187→388
- [x] #6 README.md mentions Sense→Sharpen as default pipeline
- [x] #7 cargo build succeeds after version bump
- [x] #8 Git tag v0.5.0 created and pushed
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released v0.5.0 covering 26 commits since v0.4.0 (NNFT-146 through NNFT-173).

Changes:
- Bumped workspace version 0.4.0 → 0.5.0 (Cargo.toml: workspace.package + 2 internal deps)
- Added CHANGELOG v0.5.0 section: Accuracy (Sense & Sharpen, taxonomy consolidation), Added (SenseClassifier, Model2VecResources, LabelCategoryMap, snapshot learning, --sharp-only, A/B eval), Changed (default pipeline, type count, eval metrics, test count), Fixed (L2-norm, geography protection, coordinate guard)
- Updated README.md: 169→163 type counts (8 locations), domain table corrected (technology 34→30, identity 35→31, representation 27→29), test count 187→388, model accuracy table restructured for Sense→Sharpen as default, added Sense→Sharpen bullet in Features, updated Column-Mode section to describe Sense pre-classification

Verification:
- cargo build succeeds, finetype --version shows 0.5.0
- Pre-commit hook: fmt + clippy + 357 tests pass (7 core + 98 model + 252 CLI)
- Git tag v0.5.0 pushed — release workflow building 5 platform binaries
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
