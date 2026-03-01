---
id: NNFT-174
title: 'Release v0.5.0: Sense & Sharpen pipeline, taxonomy consolidation'
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-01 09:04'
updated_date: '2026-03-01 09:06'
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
- [ ] #8 Git tag v0.5.0 created and pushed
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
