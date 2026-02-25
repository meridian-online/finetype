---
id: NNFT-134
title: Text overcall investigation — address/name false positives in SOTAB
status: To Do
assignee: []
created_date: '2026-02-25 09:50'
labels:
  - accuracy
  - discovery
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
619 SOTAB columns: generic text misclassified as full_address (341) or full_name (278). Book titles, summaries, event names being misclassified. Needs investigation: attractor addition? confidence tuning? model retraining? Root cause analysis to determine fix path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Sample SOTAB text overcall columns analysed — identify pattern of false positives
- [ ] #2 Root cause determined: attractor gap vs confidence threshold vs model confusion
- [ ] #3 Written finding with data: which fix path gives the most recovery
- [ ] #4 Follow-up implementation task created if fix is viable
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
