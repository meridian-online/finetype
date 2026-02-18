---
id: NNFT-094
title: Add Windows target to release CI matrix
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 00:14'
updated_date: '2026-02-18 01:10'
labels:
  - release
  - ci
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The release workflow currently builds for Linux (x86_64, aarch64) and macOS (x86_64, aarch64) but not Windows. Add x86_64-pc-windows-msvc to the release matrix so Windows users can install finetype.

The CI workflow already has the Windows build target infrastructure (see the DuckDB extension CI which builds Windows successfully). Need to add the Windows target with .zip archive format instead of .tar.gz, and update the Homebrew formula step to skip Windows SHA256 (Homebrew doesn't support Windows).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Windows x86_64-pc-windows-msvc target added to release.yml matrix
- [x] #2 Windows binary packaged as .zip with SHA256
- [x] #3 GitHub release includes Windows binary alongside Linux/macOS
- [x] #4 Homebrew update step still works (skips Windows)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add x86_64-pc-windows-msvc to release.yml matrix (os: windows-latest, archive: zip)
2. Update download-model.sh to handle Windows symlink fallback (readlink || cat)
3. Update Build step: add shell: bash, handle .exe binary name
4. Update Package step: support .zip format and .exe binary extension
5. Update SHA256 step: handle both .tar.gz and .zip extensions
6. Verify Upload artifact and Release globs already cover .zip
7. Verify Homebrew step already skips Windows (hardcoded target list)
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Windows x86_64-pc-windows-msvc to the release CI matrix, enabling finetype CLI distribution on all three major platforms.

Changes:
- **release.yml**: Added Windows matrix entry (os: windows-latest, archive: zip)
- **release.yml**: Added `shell: bash` to Download model, Build, Package, and SHA256 steps for cross-platform compatibility
- **release.yml**: Package step now detects Windows target for `.exe` binary name and uses `7z` for `.zip` archiving
- **release.yml**: SHA256 generation uses matrix archive format variable instead of hardcoded `.tar.gz`
- **release.yml**: Upload artifact and Release globs extended to include `.zip` patterns
- **download-model.sh**: Added `readlink || cat` fallback for Windows symlink compatibility + `\r` stripping for CRLF safety
- **Homebrew step**: Already safe — hardcodes 4 Linux/macOS targets, no Windows involvement

Commit: 874a146
All 187 tests pass.
<!-- SECTION:FINAL_SUMMARY:END -->
