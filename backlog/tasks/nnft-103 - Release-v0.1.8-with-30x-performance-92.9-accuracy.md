---
id: NNFT-103
title: Release v0.1.8 with 30x performance + 92.9% accuracy
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 07:32'
updated_date: '2026-02-18 08:29'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Cut v0.1.8 release including all changes since v0.1.7: 30x tiered inference performance, 92.9% accuracy (up from 72.6%), column mode bugfix, --bench flag, and documentation updates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version bumped to 0.1.8 in all Cargo.toml files
- [x] #2 CHANGELOG.md updated with v0.1.8 entry
- [x] #3 Tag v0.1.8 pushed to trigger release CI
- [x] #4 CI builds succeed (4 platforms)
- [x] #5 Homebrew formula updated
- [x] #6 Windows build fix: build.rs resolves models/default as text file when symlink unavailable
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Version bumped to 0.1.8, CHANGELOG updated, tag v0.1.8 pushed.
Release CI run: 22130796926
CI run: 22130795893
Awaiting builds for 5 targets (x86_64/aarch64 linux, x86_64/aarch64 macos, x86_64 windows).

Windows build failed: build.rs symlink resolution falls back to hardcoded char-cnn-v4 which doesn't exist. On Windows, git checks out symlinks as plain text files. Fixed build.rs to try read_to_string as fallback when read_link fails (mirroring download-model.sh logic). Commit f954d8b.

Deleted old v0.1.8 tag, re-tagged at f954d8b, re-pushed. New release CI run: 22131486453.

Second Windows fix: removed exists() gate entirely. On Windows, git creates file-type symlinks for models/default but the target is a directory — file symlinks to directories return false for exists(). Now tries read_link then read_to_string directly. Commit ec4cadd.

Re-tagged v0.1.8 at ec4cadd. New release CI run: 22131733864.

SHA256 generation also failed on Windows: shasum not available in Git Bash. Fixed with sha256sum fallback. Commit e0c7c04.

Third release attempt (run 22131968812): all 5 builds passed, release created. Homebrew job initially failed on timing (assets not yet available), re-ran and succeeded. All green.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released v0.1.8 with comprehensive improvements across performance, accuracy, and cross-platform support.

Key changes in v0.1.8:
- 30x tiered inference throughput (17→580 val/sec) via group-then-batch processing
- Profile accuracy lifted from 72.6% to 92.9% via header_hint_generic override
- Column mode fixed for tiered model (was char-cnn only)
- --bench flag with per-tier timing breakdown
- Windows build support: build.rs symlink-to-directory resolution, sha256sum fallback

Release details:
- All 5 targets built successfully (x86_64/aarch64 linux, x86_64/aarch64 macos, x86_64 windows)
- GitHub release created with 10 assets (5 archives + 5 SHA256 checksums)
- Homebrew formula updated automatically

Windows fixes required 3 iterations:
1. build.rs read_link fallback to read_to_string (char-cnn-v4 hardcoded path)
2. Remove exists() gate (file-type symlink to directory returns false on Windows)
3. sha256sum fallback when shasum unavailable in Git Bash

Commits: 7d93cfb, f954d8b, ec4cadd, e0c7c04"
<!-- SECTION:FINAL_SUMMARY:END -->
