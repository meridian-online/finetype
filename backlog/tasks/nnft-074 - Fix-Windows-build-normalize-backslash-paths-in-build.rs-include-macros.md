---
id: NNFT-074
title: 'Fix Windows build: normalize backslash paths in build.rs include macros'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-16 00:17'
updated_date: '2026-02-16 00:17'
labels:
  - bugfix
  - windows
  - ci
dependencies: []
priority: high
---

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 portable_path() helper added to duckdb-finetype build.rs
- [x] #2 portable_path() helper added to finetype CLI build.rs
- [x] #3 DuckDB community extensions PR #1255 updated to new commit ref
- [x] #4 Linux and macOS builds still pass
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed Windows compilation failure in both finetype and duckdb-finetype repos.

Root cause: `canonicalize()` produces backslash paths on Windows (`D:\\a\\...`) which Rust interprets as invalid escape sequences (`\\a`, `\\c`, `\\D`) inside `include_bytes!()` and `include_str!()` macros.

Fix: Added `portable_path()` helper that canonicalizes the path then replaces backslashes with forward slashes, which work on all platforms inside Rust include macros.

Additional fix in duckdb-finetype: replaced hardcoded `char-cnn-v2` model reference with dynamic resolution via `models/default` symlink or lexicographic scan of `char-cnn-*` directories.

Commits:
- noon-org/duckdb-finetype: 04aea84
- noon-org/finetype: 0de4226
- duckdb/community-extensions PR #1255: updated ref, awaiting maintainer CI approval"
<!-- SECTION:FINAL_SUMMARY:END -->
