---
id: NNFT-068
title: Add mermaid inference architecture diagram to README
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-15 05:13'
updated_date: '2026-02-15 05:25'
labels:
  - documentation
dependencies: []
references:
  - README.md
  - crates/finetype-model/src/inference.rs
  - crates/finetype-model/src/column.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Users and contributors need a clear visual explanation of how finetype processes data through its inference pipeline. Add a mermaid diagram to the README showing the stages of inference for both single-value and column modes.

The diagram should cover:
1. **Input** → raw string value(s) or CSV file
2. **Character tokenization** → char-level encoding
3. **CharCNN model** → softmax over 159 types
4. **Post-processing rules** → format-based corrections (existing: RFC 3339 vs ISO 8601, hash vs token_hex, email rescue, etc.)
5. **Pattern-gated validation** → taxonomy pattern check (planned: NNFT-064)
6. **Column-mode branch**: sampling → batch inference → vote aggregation → disambiguation rules → final type
7. **Profile mode**: CSV parsing → per-column inference → tabular output

Show the flow clearly with decision points (e.g., "Does pattern match? Yes → keep prediction. No → try next prediction"). The mermaid diagram should render correctly on GitHub.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Mermaid diagram added to README.md in a new Architecture section
- [x] #2 Diagram shows single-value inference pipeline: input → tokenize → CharCNN → post-process → pattern-gate → output
- [x] #3 Diagram shows column-mode branch: sample → batch infer → vote → disambiguate → output
- [x] #4 Diagram shows profile mode: CSV parse → per-column inference → results
- [x] #5 Diagram renders correctly on GitHub (verified via preview)
- [x] #6 Brief text accompanies the diagram explaining each stage
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Read current README Architecture section (done — lines 202-227)
2. Design mermaid flowchart showing three inference modes:
   - Single-value: Input → Tokenize → CharCNN → Post-Process → Output
   - Column-mode: Sample → Batch Infer → Vote → Disambiguate → Output
   - Profile mode: CSV → Per-Column Column-Mode → Table
3. Include decision points for post-processing and disambiguation
4. Add brief explanatory text for each pipeline stage
5. Place diagram between Architecture heading and crate table
6. Also update stale type count (152 → 159) and model reference (v2 → v4) in README
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added a mermaid flowchart diagram to README.md showing the three inference modes (single-value, column, profile) and how they compose. Each mode is a colored subgraph with clear data flow. A companion table explains each pipeline stage (tokenizer, CharCNN, post-processing, vote aggregation, disambiguation, profile) with what it does and which crate owns it.

Also updated stale numbers throughout the README to reflect v0.1.3 state:
- Type count: 152 → 159
- Model: CharCNN v2 → v4
- Accuracy: 91.97% → 91.62%
- Test count: 155 → 163
- Identity types: 25 → 32
- CLI commands: 9 → 11
- Crate test counts updated
- Model paths updated (char-cnn-v2 → char-cnn-v4)
<!-- SECTION:FINAL_SUMMARY:END -->
