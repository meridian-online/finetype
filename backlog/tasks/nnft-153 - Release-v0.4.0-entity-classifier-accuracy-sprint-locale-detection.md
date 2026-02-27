---
id: NNFT-153
title: 'Release v0.4.0: entity classifier, accuracy sprint, locale detection'
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-27 13:30'
updated_date: '2026-02-27 13:30'
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
- [ ] #8 Git tagged v0.4.0 and pushed to origin
- [ ] #9 GitHub Actions release workflow triggered and completes
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
