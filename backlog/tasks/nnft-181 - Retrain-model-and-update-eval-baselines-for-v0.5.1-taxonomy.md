---
id: NNFT-181
title: Retrain model and update eval baselines for v0.5.1 taxonomy
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-02 05:50'
updated_date: '2026-03-03 03:52'
labels:
  - taxonomy
  - v0.5.1
  - model
dependencies:
  - NNFT-180
references:
  - discovery/taxonomy-revision/BRIEF.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Final phase of taxonomy revision (v0.5.1): comprehensive model retraining and evaluation after all taxonomy changes are in place.

This task runs after all structural changes (phases 1-4) and ensures:
- Fresh training data generated covering all ~166 types
- Flat CharCNN model retrained on new taxonomy
- Sense model category labels and LabelCategoryMap updated for new finance domain
- Profile eval baselines re-established
- SOTAB and GitTables eval updated if affected
- Eval report generated and reviewed
- No regressions on existing type detection
- CLAUDE.md updated with new taxonomy count, domain structure, and version

This is the integration/validation task — it catches any issues from the structural changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Training data regenerated for full taxonomy (all ~166 types)
- [x] #2 Flat CharCNN model retrained with updated labels
- [x] #3 Sense model LabelCategoryMap updated for finance domain
- [x] #4 Profile eval passes with no regressions vs v0.5.0 baseline
- [x] #5 SOTAB eval runs cleanly (no missing label errors)
- [x] #6 eval report generated (make eval-report)
- [x] #7 CLAUDE.md updated: type count, domain list, version, architecture changes
- [x] #8 All CI checks pass (fmt, clippy, test, check)
- [x] #9 Version bumped to 0.5.1 in Cargo.toml
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Update LabelCategoryMap (+5 types, update test counts)
2. Update finetype_to_broad_category() (+3 types)
3. Regenerate Model2Vec type embeddings
4. Prepare Sense training data
5. Train Sense model
6. Train Entity classifier
7. Retrain flat CharCNN (char-cnn-v8)
8. Switch default model symlink
9. Deploy Sense model to production path
10. Clean up remap_collapsed_label()
11. Run eval and verify
12. Version bump and docs
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Retrained all models for v0.5.1 taxonomy (164 types, down from 166 after removing screen_size and ram_size).

Changes:
- CharCNN-v9 trained on 162K synthetic samples (1,000/type, 164 classes, 5 epochs, 84.4% training accuracy)
- Model2Vec type embeddings regenerated for 164 labels (was 166)
- Sense model retrained: 87.0% broad accuracy, 78.1% entity accuracy
- Entity classifier unchanged (uses high-level categories, not affected by type removals)
- Removed screen_size/ram_size from DuckDB type mapping and generator code
- LabelCategoryMap test renamed to match actual count (164)
- Dead code cleanup: removed unused `device` field from SenseDataset, suppressed unused cross_entropy_loss
- Makefile generate target updated: 100 → 1,000 samples per type
- Version bumped to 0.5.1 in workspace Cargo.toml
- CLAUDE.md updated: type count (164), model version (char-cnn-v9), eval baselines, priority order

Eval baselines established:
- Profile: 108/119 label (90.8%), 115/119 domain (96.6%)
- Actionability: 98% for correctly classified datetime columns (overall 27% due to timezone→iso_microseconds misclassification — tracked in follow-up)
- 253 tests pass, zero clippy warnings, fmt clean

Follow-up task created for accuracy improvements targeting the remaining 11 misclassifications."
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
