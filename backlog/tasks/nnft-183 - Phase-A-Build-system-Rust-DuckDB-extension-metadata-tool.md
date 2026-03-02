---
id: NNFT-183
title: 'Phase A: Build system Rust - DuckDB extension metadata tool'
status: To Do
assignee:
  - build-tools-engineer
created_date: '2026-03-02 07:23'
updated_date: '2026-03-02 07:23'
labels:
  - phase-a
  - build-system
  - duckdb
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace external Python DuckDB metadata script with pure Rust implementation.

**Objective**: Eliminate Python dependency for DuckDB extension build (Makefile:55-61).

**Work**:
1. Create `crates/finetype-build-tools/` crate:
   - Implement `append-duckdb-metadata` CLI binary
   - Parse extension `.so` binary and inject metadata
   - Integrate with `crates/finetype-duckdb/build.rs`

2. Remove external Python script from Makefile
3. Update build documentation

**Acceptance criteria**:
- `make build-release` builds DuckDB extension with metadata without calling external Python
- Extension loads in DuckDB with version metadata intact
- Falls back gracefully if tool unavailable
- Metadata format unchanged (validated against current output)

**Note**: Can run in parallel with Phase 0 spike. Does not depend on spike outcome.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Create finetype-build-tools crate with append-duckdb-metadata binary
- [ ] #2 Parse and inject DuckDB extension metadata from .so file
- [ ] #3 Integrate with finetype-duckdb/build.rs
- [ ] #4 Update Makefile to call Rust tool instead of Python script
- [ ] #5 Verify extension loads with correct metadata via duckdb CLI
- [ ] #6 Test graceful fallback if tool missing
- [ ] #7 Update DEVELOPMENT.md with build tool documentation
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
