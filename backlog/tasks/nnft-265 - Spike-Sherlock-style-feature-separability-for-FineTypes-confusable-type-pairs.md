---
id: NNFT-265
title: >-
  Spike: Sherlock-style feature separability for FineType's confusable type
  pairs
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-09 00:00'
updated_date: '2026-03-09 00:14'
labels:
  - discovery
  - accuracy
dependencies: []
references:
  - crates/finetype-model/src/features.rs
  - 'https://arxiv.org/abs/1905.10688'
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Research spike analyzing which Sherlock-style (KDD 2019) features would best discriminate FineType's 3 remaining model-level confusions: git_sha vs hash, hs_code vs decimal_number, docker_ref vs hostname. Currently we extract 32 deterministic features. Sherlock uses ~1,588. This spike identifies the top-20 most discriminative features and maps coverage gaps.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 For each of the 3 confusable pairs, document which Sherlock-style features would theoretically discriminate them
- [x] #2 Generate synthetic samples for all 6 types and compute features on real data
- [x] #3 Produce a ranked top-20 list of most discriminative features with rationale
- [x] #4 Map which features our 32-feature extractor already captures vs genuinely new signal
- [x] #5 Write findings to discovery/sense-architecture-challenge/SPIKE_A_SHERLOCK_FEATURES.md
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Read FineType's 32-feature extractor (features.rs)
2. Research Sherlock's 1,588 features via GitHub repo and paper
3. Generate/collect real samples for all 6 confusable types
4. Compute discriminative features on sample data
5. Rank top-20 features by separability
6. Map coverage gaps vs FineType's existing features
7. Write findings to discovery/sense-architecture-challenge/SPIKE_A_SHERLOCK_FEATURES.md
8. Update task with notes and final summary
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Analyzed Sherlock's 1,588-feature architecture (4 categories: 960 char distributions, 27 global stats, 200 word embeddings, 400 paragraph vectors).

Generated and analyzed real samples for all 6 types from eval datasets and taxonomy definitions:
- git_sha: 20 samples from eval/datasets/csv/new_technology.csv (all exactly 40-char hex)
- hash: 10 samples from generator (mixed MD5/SHA-1/SHA-256: 32/40/64 chars)
- hs_code: 20 samples from eval/datasets/csv/new_geography.csv (dot-separated digit groups)
- decimal_number: 20 samples from generator + taxonomy (freeform decimals)
- docker_ref: 10 samples from eval CSV (registry/namespace/image:tag)
- hostname: 10 samples from taxonomy + common domains

Key finding: The gap is NOT per-value features (our 32 are good) but column-level aggregation statistics. We compute only the mean; Sherlock computes var/min/max/kurtosis/skewness across the column.

Pair-specific findings:
1. git_sha/hash: length-agg-var is a PERFECT separator (0.0 vs 822.6). No existing rule covers this.
2. hs_code/decimal: dot-count variance + negative-sign presence + float-parseability separate well. F3 rule exists but uses only digit_ratio+dot_segments.
3. docker_ref/hostname: slash and colon presence are perfect separators. F2 rule already handles via segment_count_slash >= 1.5.

Note: git_sha and docker_ref generators appear to not emit samples via `finetype generate` despite having generator code and release_priority >= 3. Possible generator wiring bug (not blocking for this spike).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Research spike analyzing Sherlock-style (KDD 2019) feature separability for FineType's 3 remaining model-level confusions.

Findings:
- Analyzed all 1,588 Sherlock features across 4 categories and mapped them to FineType's 32-feature extractor
- Generated/collected real samples for all 6 types and computed discriminative features with measured separability
- Produced ranked top-20 feature list with rationale and coverage gap analysis

Key conclusion: FineType's per-value features already capture the right raw signals. The critical gap is **column-level aggregation statistics** -- we compute only the mean of per-value features, but Sherlock also computes variance, min, max, kurtosis, and skewness across the column.

Top discriminators by pair:
1. git_sha/hash: length-agg-var (0.0 vs 822.6 -- perfect separator)
2. hs_code/decimal: dot-count variance + minus-sign presence + float-parseability
3. docker_ref/hostname: slash/colon character presence (already handled by Rule F2)

Recommendation: Add 6-8 targeted column-level aggregate features (var/min/max for length, segment_count_dot, segment_count_slash; character presence flags for colon and minus). These require no model retraining and slot into existing F1-F3 rule framework.

Output: discovery/sense-architecture-challenge/SPIKE_A_SHERLOCK_FEATURES.md
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
