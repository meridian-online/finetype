---
id: NNFT-251
title: Evaluation — verify accuracy lift on 250-type eval suite
status: To Do
assignee: []
created_date: '2026-03-07 23:56'
labels:
  - eval
  - model
milestone: m-12
dependencies:
  - NNFT-250
references:
  - eval/profile_eval.sh
  - eval/datasets/manifest.csv
  - eval/schema_mapping.yaml
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run the full evaluation suite on the feature-augmented model and verify accuracy improvements. Key targets:

- Profile eval label accuracy >74.1% (current v14 baseline on 250 types)
- Target 80%+ label accuracy (seed exit condition)
- No regressions on previously-correct columns
- Improved precision on known confusion pairs: cpt/postal_code, hs_code/decimal_number, docker_ref/hostname
- Inference latency within 2x of current baseline

Also add sports_events.csv to eval suite as a regression dataset.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Profile eval label accuracy exceeds 74.1% on 250-type taxonomy
- [ ] #2 No previously-correct column becomes incorrect (zero regressions)
- [ ] #3 Precision improves for cpt/postal_code confusion pair
- [ ] #4 Precision improves for hs_code/decimal_number confusion pair
- [ ] #5 Precision improves for docker_ref/hostname confusion pair
- [ ] #6 Inference latency within 2x of CharCNN-only baseline
- [ ] #7 sports_events.csv added to eval suite with manifest entries
- [ ] #8 Eval report generated and committed
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
