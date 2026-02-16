---
id: NNFT-072
title: 'Fix CI: resolve clippy lints, apply cargo fmt, add pre-commit hook'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-15 10:27'
updated_date: '2026-02-15 10:28'
labels:
  - ci
  - dx
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
CI was failing on every push due to clippy lints and formatting violations. All 5 recent runs on main were red. Root cause: code was being pushed without running fmt or clippy locally.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 cargo fmt --all -- --check passes
- [x] #2 cargo clippy -- -D warnings passes
- [x] #3 cargo test passes (169 tests)
- [x] #4 make ci target runs full local CI gauntlet
- [x] #5 Pre-commit hook runs fmt + clippy + test on every commit
- [x] #6 make setup activates hooks for new clones
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed persistent CI failures (all 5 recent runs red) caused by clippy lints and formatting violations.

Clippy fixes:
- build.rs: removed useless `format!()` on string literal
- generator.rs: `0 | 1 | 2` → `0..=2` range pattern
- column.rs: manual `RangeInclusive::contains` and collapsible `str::replace`

Formatting:
- Applied `cargo fmt --all` across workspace

Developer experience:
- Added `make ci` target (fmt → clippy → test → check) to mirror CI locally
- Added `make lint` for quick fmt + clippy only
- Added `.githooks/pre-commit` hook that runs fmt, clippy, and tests before every commit
- Added `make setup` to activate hooks on new clones

Commits: c03b2c8, baca7c5
<!-- SECTION:FINAL_SUMMARY:END -->
