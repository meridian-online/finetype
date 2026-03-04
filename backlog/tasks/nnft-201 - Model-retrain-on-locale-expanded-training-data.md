---
id: NNFT-201
title: Model retrain on locale-expanded training data
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 02:32'
updated_date: '2026-03-04 08:27'
labels:
  - locale
  - training
  - accuracy
milestone: m-6
dependencies:
  - NNFT-198
  - NNFT-199
  - NNFT-200
references:
  - models/
  - crates/finetype-train/
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generate fresh training data with all locale expansions from Tasks 4-6, then retrain CharCNN.

Retrain CharCNN flat (163 classes, seed 42). Previous CLDR retrain attempt regressed (NNFT-157-161) — those issues have since been addressed.

Evaluate against v10 baselines. Upload new model to HuggingFace, update eval baselines.

Updates scope of NNFT-133 (which tracked the strategic direction).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CharCNN retrained on locale-expanded training data (seed 42)
- [x] #2 Profile eval ≥ 110/116 (no regression from v10 baseline)
- [ ] #3 Actionability ≥ 98% (no regression from v10 baseline)
- [x] #4 Model uploaded to HuggingFace hughcameron/finetype
- [x] #5 Eval report generated (make eval-report)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Generate training data: cargo run -- generate --seed 42 --samples 1000 --priority 1 → 161,000 samples
2. Train CharCNN: 10 epochs, batch_size 512, seed 42
3. Rebuild CLI with new model embedded
4. Run profile eval: eval/profile_eval.sh
5. Run actionability eval: cargo run -p finetype-eval --bin eval-actionability
6. Upload model to HuggingFace (both hughcameron/finetype and noon-org/finetype-char-cnn)
7. Generate eval report: make eval-report
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Training progression:
- 5-epoch attempt: 83.79% training accuracy. Profile 110/116 (matched v10). Actionability dropped to 95.2% — long_full_month_date misclassified as iso_8601.
- 10-epoch attempt: 88.32% training accuracy. Profile improved to 112/116 (+2pp). long_full_month_date fixed. But rfc_2822_timestamp now misclassified as iso_8601, keeping actionability at 95.4%.

Decision: Ship 10-epoch v11 as-is. Profile gain (+1.8pp) is significant. rfc_2822 regression deferred to NNFT-194.

AC #3 (actionability ≥98%) not met: 95.4% vs 98.7% target. Root cause: rfc_2822_timestamp misclassified as iso_8601 (80 values). If that one column were fixed, actionability would be 98.0%. Accepted as-is per Hugh's decision.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Retrained CharCNN flat classifier on locale-expanded training data from NNFT-198/199/200.

## What changed
- Generated 161,000 training samples (163 types × ~987 each) using expanded generators
- Trained CharCNN v11 with 10 epochs (up from v10's 5 epochs), batch_size 512, seed 42
- Final training accuracy: 88.32% (up from v10's 83.79%)
- Updated models/default symlink to char-cnn-v11
- Uploaded model to HuggingFace (noon-org/finetype-char-cnn and hughcameron/finetype)

## Results
- **Profile eval: 112/116 (96.6%)** — up from 110/116 (94.8%) with v10
  - Fixed: ean, multilingual.name, countries.sub-region (3 columns recovered)
  - New: sports_events.venue→city (arguably correct)
  - Remaining 4: utc_offset→excel_format, venue→city, countries.name→full_name, world_cities.name→full_name
- **Actionability: 95.4%** (2890/3030) — down from v10's 98.7%
  - Fixed: long_full_month_date now correctly classified (100%)
  - Regressed: rfc_2822_timestamp misclassified as iso_8601 (80 values)
  - multilingual.date still a mixed-format column (0% parse rate)

## Decision
Shipped as-is per Hugh's approval. Profile gain (+1.8pp) is the headline. rfc_2822 regression tracked in NNFT-194.

## Tests
- cargo test: 363 passed
- cargo run -- check: 163/163 types aligned
- make eval-report: report regenerated at eval/eval_output/report.md"
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
