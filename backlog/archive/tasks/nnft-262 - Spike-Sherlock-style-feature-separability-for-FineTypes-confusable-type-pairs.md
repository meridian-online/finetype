---
id: NNFT-262
title: >-
  Spike: Sherlock-style feature separability for FineType's confusable type
  pairs
status: To Do
assignee: []
created_date: '2026-03-08 21:56'
labels:
  - discovery
  - architecture
  - features
milestone: m-12
dependencies: []
references:
  - discovery/sense-architecture-challenge/FINDINGS.md
  - discovery/sense-architecture-challenge/RESPONSE_CLAUDE.md
  - discovery/sense-architecture-challenge/RESPONSE_GEMINI.md
  - crates/finetype-model/src/features.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Discovery spike to determine which of Sherlock's ~1,588 features actually discriminate between FineType's confusable type pairs.

Both architecture challenge responses (Claude and Gemini) identify expanded deterministic features as the lowest-risk, highest-certainty improvement. We currently extract 32 features (NNFT-250). The question is: which ADDITIONAL features from Sherlock's character distribution set (960 features), statistical properties, and positional patterns would help separate our specific confusion pairs?

Target confusion pairs:
- git_sha vs hash (both 40-char hex strings)
- hs_code vs decimal_number (both dot-separated digit groups)
- docker_ref vs hostname (both contain dots and slashes)

Time-box: ~4 hours.
Output: Ranked list of discriminative features with separability scores. Written finding with data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Compute expanded Sherlock-style features (character distributions, positional patterns, statistical properties) on existing eval data for the 3 confusable type pairs
- [ ] #2 Measure class separability (e.g., Fisher's discriminant ratio or similar) for each feature across the 3 pairs
- [ ] #3 Produce a ranked list of top-20 most discriminative features with separability scores
- [ ] #4 Document which features are already captured by our 32-feature extractor vs genuinely new signal
- [ ] #5 Written finding saved to discovery/sense-architecture-challenge/ with data tables
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
