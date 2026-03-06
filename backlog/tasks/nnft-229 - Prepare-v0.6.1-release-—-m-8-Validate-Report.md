---
id: NNFT-229
title: Prepare v0.6.1 release — m-8 Validate & Report
status: Done
assignee: []
created_date: '2026-03-06 05:33'
labels:
  - release-prep
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.6.1 with m-8 Validate & Report milestone: profile --validate, quality scores/grades, markdown output, quarantine samples, taxonomy fixes, actionability eval expansion (99.7%). No model retrain needed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Version bumped to 0.6.1 in all Cargo.toml files
- [ ] #2 CHANGELOG.md updated with v0.6.1 entry
- [ ] #3 CI passes (fmt, clippy, test, taxonomy check)
- [ ] #4 GitHub release created via tag push
- [ ] #5 Homebrew tap updated automatically by CI
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released FineType v0.6.1 — m-8 Validate & Report milestone. New CLI features: profile --validate, quality scores/grades, markdown output, quarantine samples. Taxonomy fixes: currency broad_type, transform stubs, format_string_alt. Actionability eval expanded to transform-based types: 99.7% overall (Tier A 96.2%, Tier B 99.8%). All 258 tests pass. CI released to GitHub, Homebrew tap auto-updated.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
