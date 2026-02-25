---
id: NNFT-125
title: 'Release v0.2.1: locale-aware validation, max-sim semantic matching'
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-25 03:11'
updated_date: '2026-02-25 03:13'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release FineType v0.2.1 with improvements since v0.2.0:

1. **NNFT-118** — Locale-aware postal code validation (14 locales) integrated into attractor demotion Signal 1.
2. **NNFT-122** — Model2Vec threshold lowered from 0.70 to 0.65 for +12 correct semantic matches.
3. **NNFT-123** — Targeted synonyms added for 5 high-value types (IANA timezone, postal code, URL, HTTP status code, MIME type).
4. **NNFT-124** — Max-sim matching with K=3 FPS representatives eliminates centroid dilution.

Profile eval: 68/74 format-detectable correct (91.9%). 263 tests passing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cargo.toml workspace version bumped to 0.2.1
- [x] #2 CHANGELOG.md has v0.2.1 section
- [x] #3 CLAUDE.md version and milestones updated
- [x] #4 cargo build succeeds
- [x] #5 cargo test passes
- [ ] #6 Tagged v0.2.1 and pushed to origin
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
