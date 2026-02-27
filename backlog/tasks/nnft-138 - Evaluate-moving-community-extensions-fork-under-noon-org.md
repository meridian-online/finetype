---
id: NNFT-138
title: Evaluate moving community-extensions fork under noon-org
status: To Do
assignee: []
created_date: '2026-02-25 18:46'
labels:
  - infrastructure
  - distribution
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The DuckDB community-extensions fork currently lives at `hughcameron/community-extensions`. Should this be moved under `noon-org/` for consistency with the rest of the Noon project infrastructure?

Context: This fork is used for submitting and maintaining the finetype DuckDB extension in the DuckDB community extensions registry. It's a fork of `duckdb/community-extensions`.

Questions to answer:
- Does it make sense organizationally for the fork to live under noon-org?
- Are there any GitHub Actions / CI implications of moving the fork?
- Would upstream PRs from a noon-org fork work the same as from hughcameron?
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Decision made: keep at hughcameron or move to noon-org
- [ ] #2 If moving: fork transferred and CI verified
- [ ] #3 README or CLAUDE.md updated to reflect final location
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
