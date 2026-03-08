---
id: NNFT-256
title: >-
  Update README for v0.6.7 — 250 types, MCP server, load command, feature
  pipeline
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-08 05:16'
updated_date: '2026-03-08 05:29'
labels:
  - documentation
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The README was last substantially updated at v0.1.7. It's now significantly outdated at v0.6.7. Needs a comprehensive refresh covering the current 250-type taxonomy, Sense→Sharpen pipeline with feature disambiguation, MCP server, load command, pure Rust training, and current accuracy numbers.

This is a rewrite of content, not structure — the README layout is good.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Type count updated to 250 across all mentions
- [x] #2 Taxonomy domain table reflects current counts (container 12, datetime 84, finance 31, geography 25, identity 34, representation 36, technology 28)
- [x] #3 Model accuracy table updated: CharCNN v14-250, Sense→Sharpen pipeline, current eval numbers (95.7% label, 97.3% domain, 99.9% actionability)
- [x] #4 CLI section documents all 10 commands including load, mcp, train
- [x] #5 Features section mentions MCP server, load command, feature-based disambiguation, 250 types
- [x] #6 Crate table includes finetype-mcp and finetype-build-tools (9 crates)
- [x] #7 Repo structure reflects current layout (finetype-mcp, finetype-build-tools, discovery/)
- [x] #8 Architecture diagram updated with feature extraction step in Sense→Sharpen pipeline
- [x] #9 Early Development banner reviewed — consider softening given v0.6.7 maturity
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Update hero section: 250 types, remove 'Early Development' or soften
2. Update Features bullets: 250 types, MCP server, load command, feature disambiguation, current accuracy
3. Update CLI section: add load, mcp, train commands
4. Update Taxonomy table: current domain counts
5. Update Performance/Model accuracy table: v14-250, current eval numbers
6. Update Architecture diagram: add feature extraction step
7. Update crate table: add finetype-mcp, finetype-build-tools
8. Update repo structure: add missing dirs
9. Review all type count mentions (should be 250 everywhere)
10. Commit
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Comprehensive README rewrite:
- Removed 'Early Development' banner (AC #9) — v0.6.7 is mature enough
- Added MCP server section with tool table and resources
- Added feature disambiguation to pipeline diagram and stages table
- Updated all type counts to 250, domain counts to current
- Added load, mcp, train to CLI examples
- Added finetype-mcp, finetype-build-tools to crate table (9 crates)
- Added discovery/ to repo structure
- Removed validate command section (was stale/not current)
- Removed GitTables accuracy section (numbers were for v0.1.x era)
- Kept DuckDB strptime limitation section (still relevant)
- Only stale '164' reference is in Tiered v2 row (correct — legacy model)
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Comprehensive README rewrite for v0.6.7. Updated from v0.1.7-era content to reflect current 250-type taxonomy, Sense→Sharpen pipeline with feature disambiguation, MCP server, load command, pure Rust training, and current accuracy numbers (95.7% label, 99.9% actionability).

Key changes:
- Removed 'Early Development' banner
- Added MCP server section with 6-tool table and resource URIs
- Added feature extraction + disambiguation to architecture diagram and pipeline stages
- Updated all type counts (250), domain counts, model accuracy table (v14-250)
- CLI section now documents all 10 commands including load, mcp, train
- Crate table expanded to 9 crates (added finetype-mcp, finetype-build-tools)
- Repo structure updated with discovery/, finetype-mcp/, finetype-build-tools/
- Removed stale validate command section and GitTables accuracy numbers
- Kept DuckDB strptime locale limitation (still relevant)

No code changes — documentation only.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
