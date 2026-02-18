---
id: NNFT-096
title: Update README with tiered model architecture documentation
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 00:14'
updated_date: '2026-02-18 01:18'
labels:
  - documentation
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The README needs updating to reflect the tiered model architecture that shipped in v0.1.7. Currently it likely documents the flat CharCNN approach. Update to cover:
- Tiered inference hierarchy (T0 broad type → T1 category → T2 specific type)
- The --model-type flag
- Updated accuracy numbers (72.6% format-detectable label accuracy)
- Column-mode disambiguation rules including SI number override
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 README describes tiered model architecture
- [x] #2 Accuracy numbers updated to reflect tiered-v2 profile eval results
- [x] #3 CLI examples show default tiered behavior
- [x] #4 --model-type flag documented
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Update model references from char-cnn-v6 to tiered-v2
2. Update Features bullet: CharCNN → tiered CharCNN
3. Update model accuracy table with tiered-v2 row
4. Update Architecture section: describe tiered T0→T1→T2 hierarchy
5. Update Mermaid diagram for tiered inference
6. Update pipeline stages table
7. Document --model-type flag in CLI examples
8. Update test count (213 → 187)
9. Update repository structure (models/char-cnn-v6 → models/tiered-v2)
10. Verify library API example still correct
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Updated README to reflect the tiered model architecture shipped in v0.1.7.

Changes:
- **Features**: Updated inference bullet from "Character-level CNN" to "34 specialized CharCNN models in a T0→T1→T2 hierarchy"
- **CLI examples**: Added `--model-type` flag example showing how to use flat char-cnn models vs default tiered
- **Model accuracy table**: Added Tiered v2 row with architecture column; flat models labelled as "Flat (single model)"
- **Mermaid diagram**: Rewrote single-value mode to show Tier 0 → Tier 1 → Tier 2 cascade instead of flat softmax
- **Pipeline stages**: Updated CharCNN row to describe tiered architecture (34 specialized models, T0→T1→T2)
- **Crate table**: finetype-model description updated to "Tiered CharCNN inference" with 114 tests
- **Repository structure**: models/char-cnn-v6 → models/tiered-v2 with updated description
- **Architecture section**: Added "Why Tiered CharCNNs?" explanation of the cascade approach
- **Test count**: 213 → 187 (73 core + 114 model)
- **All model path references**: char-cnn-v6 → tiered-v2

Commit: dab8ddc"
<!-- SECTION:FINAL_SUMMARY:END -->
