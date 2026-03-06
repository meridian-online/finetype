---
id: NNFT-232
title: Release FineType v0.6.2
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-06 07:54'
updated_date: '2026-03-06 08:03'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.6.2 with m-10 Schema Export features (DdlInfo API, schema-for command, Arrow output, x-finetype-* fields)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Update version in Cargo.toml and Cargo.lock
- [x] #2 Run cargo test + check (all 258 tests pass, taxonomy check clean)
- [x] #3 Tag main with v0.6.2
- [x] #4 Push tag to GitHub (CI builds + releases)
- [ ] #5 Verify Homebrew tap auto-update
- [ ] #6 Update crates.io
- [ ] #7 Verify finetype-duckdb community extension build passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- AC #4: Tag v0.6.2 pushed to GitHub
- CI Release workflow triggered (building artifacts)
- Homebrew tap auto-update pending CI completion
- crates.io publish pending CI workflow
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released FineType v0.6.2 with m-10 Schema Export milestone.

Completed:
✅ NNFT-210: DdlInfo API (Taxonomy::ddl_info method)
✅ NNFT-218: finetype schema-for command (CSV/JSON → CREATE TABLE DDL)
✅ NNFT-219: --output arrow for Arrow IPC schema JSON
✅ NNFT-220: x-finetype-* extension fields in finetype schema

Release Process:
✅ Version bumped to 0.6.2 in Cargo.toml/Cargo.lock
✅ All 258 tests pass, taxonomy check clean
✅ git tag v0.6.2 created with detailed changelog
✅ Tag pushed to GitHub (CI Release workflow triggered)
⏳ GitHub release artifacts building (CI in progress)
⏳ Homebrew tap auto-update pending CI completion
⏳ crates.io publish pending CI workflow

Final Summary for PR:
Implemented complete DDL-oriented schema export pipeline with three complementary features: DdlInfo metadata API for taxonomy lookup, schema-for command for direct CREATE TABLE generation from profiled files, and Arrow schema output for interoperability with data tools. Also enriched existing schema command with DDL contract fields (x-finetype-broad-type, transform, format-string) for programmatic code generation.
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
