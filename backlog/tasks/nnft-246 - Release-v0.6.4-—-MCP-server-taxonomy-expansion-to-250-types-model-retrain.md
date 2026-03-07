---
id: NNFT-246
title: 'Release v0.6.4 — MCP server, taxonomy expansion to 250 types, model retrain'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 11:15'
updated_date: '2026-03-07 11:31'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Cut release v0.6.4 with all changes since v0.6.3: MCP server (NNFT-241), taxonomy precision cleanup (NNFT-242/243), taxonomy expansion to 250 types (NNFT-244), and full pipeline retrain (NNFT-245).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version bumped to 0.6.4 in workspace Cargo.toml
- [x] #2 CHANGELOG.md updated with v0.6.4 entry
- [x] #3 cargo test + finetype check pass
- [x] #4 Tagged v0.6.4 and pushed to trigger release workflow
- [x] #5 CI release workflow completes successfully
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Bump version to 0.6.4 in workspace Cargo.toml
2. Write CHANGELOG.md entry
3. Update CLAUDE.md version
4. cargo test + finetype check
5. Commit, tag v0.6.4, push
6. Monitor CI release workflow
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released FineType v0.6.4 to production.

## What shipped
- **MCP server** (`finetype mcp`) — 6 tools + taxonomy resources via rmcp v1.1.0 (NNFT-241)
- **Taxonomy expansion** — 207→250 types (+43 definitions across all domains) (NNFT-244)
- **Taxonomy precision** — Removed http_status_code/port, renamed currency amounts (NNFT-242/243)
- **Full model retrain** — CharCNN-v14-250, Sense, Model2Vec for 250-type taxonomy (NNFT-245)
- **PII field** + `x-finetype-pii`/`x-finetype-transform-ext` in schema output

## Release artifacts
- 5 platform builds: Linux x86/arm, macOS x86/arm, Windows
- Homebrew tap formula updated automatically
- All CI jobs passed (build, release, homebrew)

## Tests
- cargo test: 405 pass
- finetype check: 250/250 (100%)
- CI: all green
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
