---
id: NNFT-101
title: Fix column mode crash with tiered default model
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 06:08'
updated_date: '2026-02-18 06:08'
labels:
  - bugfix
  - cli
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Column mode (--mode column) in the infer command was hardcoded to only work with --model-type char-cnn. After NNFT-087 changed the default model to tiered, the smoke test CI started failing because column mode would exit with error on the tiered model type.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Column mode works with tiered model type (default)
- [x] #2 Column mode works with all model types (char-cnn, tiered, transformer)
- [x] #3 Smoke tests pass (all 25)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Replaced the model-type-specific column mode handler with a generic one using Box<dyn ValueClassifier>. All three model types (char-cnn, tiered, transformer) now work with --mode column. Root cause: column mode was added when char-cnn was the default, but NNFT-087 changed the default to tiered without updating the column mode match arm. Commit fa29ed4.
<!-- SECTION:FINAL_SUMMARY:END -->
