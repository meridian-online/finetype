---
id: NNFT-164
title: 'Phase 2: Sense → Sharpen Integration Design'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-28 23:02'
updated_date: '2026-02-28 23:11'
labels: []
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Produce the integration design spec for Phase 3 (Rust implementation) of the Sense & Sharpen pivot. Creates a design document covering pipeline flow, type mapping, rule survival, Rust interfaces, shared Model2Vec, build system, and verification. Also creates Phase 3 backlog tasks and a decision record.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Design document written at discovery/architectural-pivot/PHASE2_DESIGN.md
- [x] #2 All 163 FineType types mapped to exactly one Sense category
- [x] #3 Rule survival analysis: each of 17 rules categorised as retained/absorbed/safety-net
- [x] #4 Rust interface design covers SenseClassifier, Model2VecResources, modified ColumnClassifier
- [x] #5 Phase 3 backlog tasks created (7-9 atomic tasks)
- [x] #6 Decision recorded as decision-006
- [x] #7 CLAUDE.md updated with Phase 2 completion
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Phase 2 Integration Design for Sense → Sharpen pivot (decision-006).

Delivered:
- Design document at discovery/architectural-pivot/PHASE2_DESIGN.md (~500 lines) covering: new pipeline flow (Sense → flat CharCNN → masked vote → 12 rules → locale), all 163 types mapped to 6 Sense categories, rule survival analysis (12 retained, 4 reduced scope, 6 absorbed by Sense), Rust interface designs (SenseClassifier, Model2VecResources, LabelCategoryMap, modified ColumnClassifier), Architecture A Candle port spec, shared Model2Vec architecture (+1.4MB net), build system additions, verification plan, migration path, open questions
- 8 Phase 3 implementation tasks (NNFT-165 through NNFT-172) with dependencies and acceptance criteria
- Decision-006: flat CharCNN + output masking over per-category retraining, sample 100/encode 50, Sense absorbs 6 behaviours
- CLAUDE.md updated: new milestone, priority order, decided item #19, key file reference

Key design choices:
1. Flat CharCNN + output masking (not tiered, not per-category retrain) — simpler, avoids regressions
2. Sample 100 values for CharCNN, encode first 50 for Sense — matches spike training config
3. Sense absorbs header hints, entity demotion, geography protection — 6 behaviours eliminated
4. Shared Model2VecResources avoids triple-loading 7.4MB embedding matrix
5. Backward-compatible fallback when Sense model absent
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
