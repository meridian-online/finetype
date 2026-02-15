---
id: NNFT-064
title: Implement pattern-gated post-processing using taxonomy validation patterns
status: To Do
assignee: []
created_date: '2026-02-15 05:12'
labels:
  - feature
  - inference
  - post-processing
dependencies: []
references:
  - crates/finetype-model/src/inference.rs
  - crates/finetype-model/src/column.rs
  - crates/finetype-cli/build.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
126 of 159 taxonomy types have validation patterns (regex). These patterns are already embedded at compile time but are only used for `finetype check`. They should also gate inference results.

The idea: after the model predicts a type, check whether the input actually matches that type's validation pattern. If it doesn't, fall back to the next-best prediction from the model's softmax output.

Implementation levels:
1. **Single-value mode**: Model predicts iata_code for "C85" → check against `^[A-Z]{3}$` → fails → try 2nd prediction
2. **Column mode**: Model votes iata_code as majority → check what fraction of values match the pattern → if below threshold (e.g., <50%), reject and promote next candidate

This requires:
- Making taxonomy validation patterns available to the inference/post-processing pipeline
- Extending `post_process()` or adding a new `validate_prediction()` step
- Loading patterns from embedded taxonomy at classifier initialization
- Careful ordering: pattern validation should run AFTER existing post-processing rules (which handle known confusion pairs) but BEFORE returning final results

This is a general-purpose improvement that catches many false positives without per-type rules.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Taxonomy validation patterns are loaded and available during inference
- [ ] #2 Single-value post-processing checks prediction against pattern and falls back to next-best if mismatch
- [ ] #3 Column-mode checks fraction of values matching pattern and rejects type if below 50%
- [ ] #4 Titanic Cabin column no longer classified as iata_code (pattern rejects C85, E46 etc)
- [ ] #5 Existing post-processing rules still take priority (they handle known confusion pairs)
- [ ] #6 No regression on eval accuracy (pattern gating should only reject genuine mismatches)
- [ ] #7 New unit tests for pattern validation fallback behavior
<!-- AC:END -->
