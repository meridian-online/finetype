---
id: NNFT-178
title: >-
  Create representation.identifier category (move UUID, alphanumeric_id,
  increment)
status: Done
assignee: []
created_date: '2026-03-02 05:50'
updated_date: '2026-03-02 06:23'
labels:
  - taxonomy
  - v0.5.1
dependencies: []
references:
  - discovery/taxonomy-revision/BRIEF.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 2 of taxonomy revision (v0.5.1): create a new `representation.identifier` category grouping types that indicate "this column is a key/identifier."

Move three existing types:
- `technology.cryptographic.uuid` → `representation.identifier.uuid`
- `representation.code.alphanumeric_id` → `representation.identifier.alphanumeric_id`
- `representation.numeric.increment` → `representation.identifier.increment`

Rationale: UUID appears in database design far beyond technology contexts. Increment (monotonic increasing) is a fundamental database concept. Alphanumeric IDs are identifiers by definition. Grouping them helps analysts understand "this column is a key."

The `representation.code` category becomes empty after this move and should be removed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Three types moved to representation.identifier in YAML definitions
- [x] #2 representation.code category removed (empty after move)
- [x] #3 All label references updated across codebase (LabelCategoryMap, Sense categories, training data, eval)
- [x] #4 cargo run -- check passes
- [x] #5 cargo test passes
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Created representation.identifier category grouping UUID, alphanumeric_id, and increment — types that signal "this column is a key."

Moves:
- technology.cryptographic.uuid → representation.identifier.uuid
- representation.code.alphanumeric_id → representation.identifier.alphanumeric_id
- representation.numeric.increment → representation.identifier.increment
- representation.code category removed (empty after move)

All Rust label references updated (label_category_map.rs, column.rs, inference.rs, type_mapping.rs). Zero stale references remain.

Note: ACs #6-7 (model retrain, eval baselines) deferred to NNFT-181.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
