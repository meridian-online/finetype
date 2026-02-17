---
id: NNFT-021
title: Submit DuckDB extension to community extensions
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-10 05:32'
updated_date: '2026-02-17 22:39'
labels:
  - release
  - duckdb
milestone: 'Phase 6: Open Source & HuggingFace'
dependencies:
  - NNFT-017
  - NNFT-019
references:
  - DEVELOPMENT.md
  - 'https://github.com/duckdb/community-extensions'
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Submit the finetype DuckDB extension to the DuckDB community extensions repository so users can install via `INSTALL finetype FROM community`. Requires the extension to pass DuckDB's CI/testing requirements, include documentation, and work across supported platforms (Linux, macOS, Windows).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Extension builds on Linux, macOS, and Windows
- [x] #2 Extension passes DuckDB community extensions CI requirements
- [x] #3 PR submitted to duckdb/community-extensions with extension manifest
- [x] #4 Documentation includes: installation, function reference, examples
- [x] #5 INSTALL finetype FROM community works after merge
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Create noon-org/duckdb-finetype repo on GitHub
2. Initialize with standalone Cargo.toml (crate name: finetype, depends on finetype-core + finetype-model from crates.io)
3. Copy/adapt source from crates/finetype-duckdb/ (lib.rs, type_mapping.rs, normalize.rs, unpack.rs, build.rs)
4. Add extension-ci-tools as git submodule (v1.4.4 tag)
5. Add Makefile following the dns extension pattern (includes base.Makefile + rust.Makefile)
6. Add .cargo/config.toml for Windows static CRT linking
7. Add SQL test files (test/sql/finetype.test)
8. Add MainDistributionPipeline.yml GitHub Actions workflow
9. Test build locally
10. Fork duckdb/community-extensions, add extensions/finetype/description.yml, submit PR
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Created standalone DuckDB extension repo: https://github.com/noon-org/duckdb-finetype

Architecture:
- Standalone Cargo.toml (crate name: finetype) depends on finetype-core + finetype-model from crates.io
- Model files embedded at compile time via build.rs (same as workspace version)
- extension-ci-tools v1.4.4 as git submodule
- Makefile includes base.Makefile + rust.Makefile for community CI compatibility
- .cargo/config.toml for Windows static CRT linking

Build verified:
- `cargo build --release` produces libfinetype.so (2.2MB)
- `make configure release` produces finetype.duckdb_extension with metadata
- Extension loads in DuckDB v1.4.4 with -unsigned flag
- All 5 functions work: finetype(), finetype_detail(), finetype_cast(), finetype_unpack(), finetype_version()
- SQL logic tests pass via `make test_release`

Excluded platforms: wasm_mvp, wasm_eh, wasm_threads, linux_amd64_musl (same as dns/encoding/lsh extensions)

PR submitted: https://github.com/duckdb/community-extensions/pull/1255

AC #1 (builds on Linux, macOS, Windows): Builds locally on Linux. CI will test macOS and Windows when PR runs.
AC #2 (CI requirements): Pending PR CI run.
AC #5 (INSTALL FROM community): Pending PR merge."

PR #1255 merged on Feb 16, 2026. All CI checks passed: Linux (amd64, arm64), macOS (amd64, arm64), Windows (amd64, mingw). Extension deployed to community repository. `INSTALL finetype FROM community` now works.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
DuckDB community extension submitted and merged. Extension available via `INSTALL finetype FROM community` on DuckDB v1.4.4+.

Changes:
- Created standalone repo: https://github.com/noon-org/duckdb-finetype
- Standalone Cargo.toml depends on finetype-core + finetype-model from crates.io
- Model files embedded at compile time via build.rs
- extension-ci-tools v1.4.4 as git submodule
- All 5 functions: finetype(), finetype_detail(), finetype_cast(), finetype_unpack(), finetype_version()
- Builds on Linux (amd64, arm64), macOS (amd64, arm64), Windows (amd64, mingw)
- PR #1255: https://github.com/duckdb/community-extensions/pull/1255 — merged Feb 16"
<!-- SECTION:FINAL_SUMMARY:END -->
