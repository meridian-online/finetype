---
id: NNFT-246
title: 'Release v0.6.4 — MCP server, taxonomy expansion to 250 types, model retrain'
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-07 11:15'
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
- [ ] #1 Version bumped to 0.6.4 in workspace Cargo.toml
- [ ] #2 CHANGELOG.md updated with v0.6.4 entry
- [ ] #3 cargo test + finetype check pass
- [ ] #4 Tagged v0.6.4 and pushed to trigger release workflow
- [ ] #5 CI release workflow completes successfully
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
