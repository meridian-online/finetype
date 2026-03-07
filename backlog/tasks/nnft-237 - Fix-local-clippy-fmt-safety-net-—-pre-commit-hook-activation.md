---
id: NNFT-237
title: Fix local clippy/fmt safety net — pre-commit hook activation
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-06 21:09'
updated_date: '2026-03-06 21:09'
labels:
  - dx
  - ci
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
8+ fix-up commits for clippy/fmt failures were caught by CI but not locally. Two root causes:

1. Local Rust could fall behind CI's `@stable` — new lints appear in CI first
2. Pre-commit hook exists at `.githooks/pre-commit` but `core.hooksPath` pointed to `.git/hooks/` (default, no hook there)

Fixed by updating `make setup` to update Rust stable and set `core.hooksPath` correctly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 make setup updates Rust to latest stable (rustup or brew, whichever is available)
- [x] #2 make setup sets core.hooksPath to .githooks
- [x] #3 Pre-commit hook runs cargo fmt --check + cargo clippy -D warnings + cargo test before every commit
- [x] #4 make ci passes locally
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Updated `make setup` target in Makefile to:
1. Run `rustup update stable` (or `brew upgrade rust` on Homebrew installs) to keep local Rust current with CI
2. Set `git config core.hooksPath .githooks` so the existing pre-commit hook actually fires

The pre-commit hook at `.githooks/pre-commit` already ran fmt + clippy + test — it just wasn't being activated because `core.hooksPath` defaulted to `.git/hooks/`. No other file changes needed.

Verified: `git config core.hooksPath` → `.githooks`, `make ci` passes.
<!-- SECTION:FINAL_SUMMARY:END -->
