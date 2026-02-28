---
id: NNFT-168
title: Port Sense Architecture A to Candle/Rust
status: To Do
assignee: []
created_date: '2026-02-28 23:06'
labels:
  - sense-sharpen
  - feature
dependencies:
  - NNFT-165
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement SenseClassifier in a new sense.rs module. Ports PyTorch SenseModelA (cross-attention over Model2Vec) to Candle. Loads safetensors weights matching the Phase 1 spike model. Includes BroadCategory/EntitySubtype enums and SenseResult struct.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 New sense.rs module with SenseClassifier struct
- [ ] #2 from_bytes() loads safetensors with correct weight mapping (QKV split from in_proj)
- [ ] #3 classify() produces correct BroadCategory and EntitySubtype
- [ ] #4 Forward pass matches PyTorch output within 1e-4 tolerance on test inputs
- [ ] #5 BroadCategory and EntitySubtype enums with Display/From impls
- [ ] #6 Unit tests with synthetic tensors
- [ ] #7 Integration test loads real spike model artifacts and classifies sample columns
- [ ] #8 Exported from lib.rs
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
