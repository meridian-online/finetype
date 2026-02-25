---
id: NNFT-136
title: 'Expand phone locale patterns — extensions, more countries'
status: To Do
assignee: []
created_date: '2026-02-25 11:39'
labels:
  - accuracy
  - validation
dependencies:
  - NNFT-132
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
NNFT-132 investigation found that only 23/254 SOTAB cardinality-demoted phone columns pass existing locale patterns. The validation precision fix (gating Signal 3 with locale_confirmed) rescues those 23, but the remaining 231 need expanded locale coverage.

Specific gaps identified in SOTAB data:
- Phone extensions: '(847) 945-4636 Work', '615.966.6280 (ext. 6280)', '(239) 939-1400 Ext. 103', '201-880-7213 x117'
- German (0) trunk prefix: '+49 (0)7721 - 90 88 70'
- South African numbers: '041 364 2260', '+27 215117818' (no ZA locale)
- German en-dash separators: '089 – 218 965 757'

Additionally, 14 SOTAB columns were demoted by Signal 1 (validation failure) due to these format gaps.

Fix approach: Add optional extension suffix to all locale patterns, add ZA locale, review DE pattern for (0) trunk prefix.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 EN_US/EN_CA patterns accept optional extension suffix (ext/Ext/x/Work/Cell)
- [ ] #2 DE pattern handles (0) trunk prefix notation
- [ ] #3 ZA locale pattern added for South African phone numbers
- [ ] #4 All tests pass — cargo test + taxonomy check
- [ ] #5 SOTAB re-eval shows measurable improvement in telephone column accuracy
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
