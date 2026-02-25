---
id: NNFT-132
title: Phone validation investigation — SOTAB telephone demotion to categorical
status: To Do
assignee: []
created_date: '2026-02-25 09:17'
labels:
  - accuracy
  - validation
  - discovery
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
634 SOTAB telephone columns are being demoted from phone_number to categorical by attractor demotion rules. Actual values include formats like (661) 284-3600, 05 61 85 61 48, 07584674902.

This is a discovery spike to determine:
- Which phone formats in SOTAB aren't covered by our 14 locale patterns?
- Is it a pattern-coverage gap (add more locale patterns) or something deeper (cardinality demotion firing inappropriately)?
- What's the fix: more locale patterns, relaxed validation, or cardinality threshold adjustment?

Time budget: ~2-4 hours investigation, produce written finding with data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Sample SOTAB phone columns analysed — identify which formats fail current locale validation
- [ ] #2 Root cause determined: pattern gap vs cardinality demotion vs confidence threshold
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
