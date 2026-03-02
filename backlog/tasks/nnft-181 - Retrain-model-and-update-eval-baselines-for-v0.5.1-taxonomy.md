---
id: NNFT-181
title: Retrain model and update eval baselines for v0.5.1 taxonomy
status: To Do
assignee: []
created_date: '2026-03-02 05:50'
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
- [ ] #1 Training data regenerated for full taxonomy (all ~166 types)
- [ ] #2 Flat CharCNN model retrained with updated labels
- [ ] #3 Sense model LabelCategoryMap updated for finance domain
- [ ] #4 Profile eval passes with no regressions vs v0.5.0 baseline
- [ ] #5 SOTAB eval runs cleanly (no missing label errors)
- [ ] #6 eval report generated (make eval-report)
- [ ] #7 CLAUDE.md updated: type count, domain list, version, architecture changes
- [ ] #8 All CI checks pass (fmt, clippy, test, check)
- [ ] #9 Version bumped to 0.5.1 in Cargo.toml
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
