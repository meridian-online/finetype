---
id: NNFT-135
title: Numeric age/integer disambiguation investigation
status: Done
assignee: []
created_date: '2026-02-25 09:50'
updated_date: '2026-03-03 12:59'
labels:
  - accuracy
  - discovery
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
205 SOTAB columns: generic numbers classified as age at 0.995 confidence. Small integers (5, 10, 4, 2) are genuinely ambiguous without header context. Model is very confident; rules can't help when confidence is near-perfect. Probably requires model-level changes or cross-column context. Investigation spike to determine if any rule-based approach is viable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Sample SOTAB age overcall columns analysed — confirm values are genuinely ambiguous
- [ ] #2 Assess whether header hints could rescue these (check if columns have informative headers)
- [ ] #3 Written finding: rule-fixable vs model-level vs cross-column context needed
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Resolved by removal of `identity.person.age` from the taxonomy in NNFT-192. The age type was indistinguishable from plain integers (CAST(col AS SMALLINT) is identical to integer_number), and produced 205 SOTAB false positives. Age columns now correctly fall through to integer_number, which is the appropriate type for plain numeric values without format-specific validation.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
