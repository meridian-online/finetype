---
id: NNFT-142
title: Add type alias/duplicate metadata for vote merging in column classification
status: To Do
assignee: []
created_date: '2026-02-26 00:28'
labels:
  - taxonomy
  - disambiguation
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The old finetype prototype marked certain types as duplicates of others (name → first_name, surname → last_name, telephone → phone_number). Add an aliases or merge_with field to taxonomy definitions to document type relationships, then optionally merge votes from closely related types during column vote aggregation.

This makes type relationships explicit and available to the disambiguation pipeline. Lower priority than designation gating and locale detection.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Taxonomy YAML supports an aliases or merge_with field on type definitions
- [ ] #2 finetype-core parses alias relationships without error
- [ ] #3 At least 3 known type relationships are documented in the taxonomy
- [ ] #4 Column vote aggregation merges votes from aliased types before majority vote
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
