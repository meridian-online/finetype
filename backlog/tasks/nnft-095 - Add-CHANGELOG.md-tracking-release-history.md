---
id: NNFT-095
title: Add CHANGELOG.md tracking release history
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 00:14'
updated_date: '2026-02-18 01:12'
labels:
  - documentation
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The project has no CHANGELOG.md. Each release should have a summary of changes. Retroactively create the file covering v0.1.0 through v0.1.7, then maintain it going forward.

Use Keep a Changelog format (https://keepachangelog.com/) with sections: Added, Changed, Fixed, Removed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CHANGELOG.md created with entries for v0.1.0 through v0.1.7
- [x] #2 Follows Keep a Changelog format
- [x] #3 Each release entry summarizes key changes from backlog tasks
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Review existing CHANGELOG.md (has v0.1.0, v0.1.2, v0.1.3, v0.1.5)
2. Add missing entries: v0.1.1, v0.1.4, v0.1.6, v0.1.7
3. Update [Unreleased] section with post-v0.1.7 changes
4. Verify Keep a Changelog format compliance
5. Commit and push
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Completed CHANGELOG.md with full release history from v0.1.0 through v0.1.7. The file previously existed with entries for v0.1.0, v0.1.2, v0.1.3, and v0.1.5.

Changes:
- Added missing entries for v0.1.1 (embedded model, crates.io), v0.1.4 (17 new types, pattern post-processing), v0.1.6 (evaluation pipeline, benchmark datasets), and v0.1.7 (tiered model as default)
- Added [Unreleased] section covering Windows CI target, DuckDB extension v0.2.0, and crate publishing
- Added Keep a Changelog header with format and semver links
- Each entry references the relevant backlog task IDs (NNFT-xxx)
- All 8 releases now documented with accurate dates from git tags

Commit: 4271b84"
<!-- SECTION:FINAL_SUMMARY:END -->
