---
id: NNFT-133
title: Model retraining with CLDR locale data for improved type accuracy
status: To Do
assignee: []
created_date: '2026-02-25 09:17'
updated_date: '2026-02-26 01:41'
labels:
  - accuracy
  - training
  - strategic
dependencies:
  - NNFT-058
  - NNFT-060
  - NNFT-126
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
NNFT-130 evaluation confirms that 63% of SOTAB format-detectable errors (5,039 columns) are in the "other" long tail — not addressable by disambiguation rules. The tiered CharCNN model's accuracy ceiling is the bottleneck.

This is the strategic investment path. Depends on:
- NNFT-058: CLDR date/time format permutation
- NNFT-060: CLDR release data integration

NOTE: decision-002 (accepted) rules out 4-level locale-in-label training for the CharCNN. The model retrains here focus on better TYPE accuracy — more diverse training data from CLDR locale sources, not locale classification. Locale detection is handled post-hoc via validation_by_locale (NNFT-140, shipped).

Key evidence from NNFT-130:
- Domain accuracy roughly comparable between flat (v0.1.8) and tiered (v0.3.0) — model quality, not rule coverage, is the limiting factor
- Biggest model-level confusions: text↔address↔name (619 cols), numeric age overcall at 0.995 confidence (205 cols), Duration/SEDOL pattern confusion (partially addressed by rules)
- Header hints provide 51.1% domain accuracy on GitTables semantic_only tier — model needs to match this without headers

This task tracks the strategic direction. Actual implementation through NNFT-058 and NNFT-060.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CLDR locale data integrated as upstream source (NNFT-060)
- [ ] #2 Date/time formats permuted by locale using CLDR patterns (NNFT-058)
- [ ] #3 SOTAB and GitTables re-evaluated showing measurable improvement over v0.3.0 baseline
- [ ] #4 CharCNN models retrained with CLDR-diversified training data (more locale-representative samples per type, 3-level labels)
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
