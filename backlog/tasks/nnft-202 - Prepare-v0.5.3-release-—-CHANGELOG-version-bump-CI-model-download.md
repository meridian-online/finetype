---
id: NNFT-202
title: 'Prepare v0.5.3 release — CHANGELOG, version bump, CI model download'
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-04 10:22'
updated_date: '2026-03-04 10:24'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Ship locale expansion (NNFT-195–201) and accuracy recovery (NNFT-194) as v0.5.3.

Requires: version bump, CHANGELOG entry, Sense model download in CI script (missing — Sense is embedded at build time via include_bytes but download-model.sh doesn't fetch it yet), commit, tag, push.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version bumped to 0.5.3 in workspace Cargo.toml
- [x] #2 CHANGELOG.md updated with v0.5.3 entry covering NNFT-194 through NNFT-201
- [x] #3 download-model.sh downloads Sense model from HuggingFace
- [x] #4 All models verified on HuggingFace (char-cnn-v11, model2vec, entity-classifier, sense)
- [x] #5 cargo test passes
- [ ] #6 Commit tagged and pushed to trigger CI/release workflow
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Bump version 0.5.2 → 0.5.3 in workspace Cargo.toml
2. Add Sense model download section to .github/scripts/download-model.sh
3. Write CHANGELOG.md entry for v0.5.3 (locale expansion + accuracy recovery)
4. Update CLAUDE.md version string
5. Run cargo test to verify
6. Commit all changes with task ID
7. Tag v0.5.3 and push to trigger CI/release
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
