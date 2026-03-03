---
id: NNFT-193
title: 'Prepare v0.5.2 release — format, CHANGELOG, model upload'
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-03 22:34'
updated_date: '2026-03-03 22:36'
labels:
  - release
  - v0.5.2
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Fix three blockers preventing v0.5.2 release after NNFT-192 (taxonomy revision).

**Blockers:**

1. **CI broken** — `cargo fmt` violations in 4 files (generator.rs, type_mapping.rs, column.rs, data.rs). Last NNFT-192 commit failed CI check.

2. **Missing CHANGELOG entry** — Need [0.5.2] section documenting taxonomy changes (removed street_number, age; added numeric_code), actionability improvement (92.7%→98.7%), profile eval baseline update.

3. **char-cnn-v10 model not on HuggingFace** — NNFT-192 retrained to v10, but only v9 was uploaded last session. Release CI will fail on model download. Need to upload char-cnn-v10 to HuggingFace.

**Known issues (documented, not blockers):**
- Profile eval regression: 117/119 (98.3%)→110/116 (94.8%) due to retrain boundary shifts. 6 misclassifications. Defer to v0.5.3 follow-up task.

**Release timeline:**
After this task completes → tag v0.5.2 → trigger release workflow → ship.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 cargo fmt applied to all modified files; CI checks pass (fmt+clippy+test+check)
- [x] #2 CHANGELOG entry [0.5.2] added with taxonomy changes, actionability improvement, model retrain summary
- [x] #3 char-cnn-v10 model uploaded to HuggingFace hughcameron/finetype; verified accessible
- [x] #4 All blockers resolved; ready to tag v0.5.2 and trigger release workflow
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Run cargo fmt to fix formatting violations in generator.rs, type_mapping.rs, column.rs, data.rs
2. Update CHANGELOG.md with [0.5.2] section documenting taxonomy changes and improvements
3. Upload char-cnn-v10 model to HuggingFace hughcameron/finetype
4. Verify CI passes (fmt+clippy+test+check)
5. Update NNFT-192 task file metadata if needed
6. Commit all changes with task ID
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Step 1: cargo fmt — all violations fixed ✓

Step 2: CHANGELOG — [0.5.2] section added with taxonomy changes, actionability improvement (96.0%→98.7%), profile eval regression (98.3%→94.8%), model retrain summary ✓

Step 3: char-cnn-v10 model uploaded to HuggingFace hughcameron/finetype ✓ — verified download works

Step 4: CI passes — fmt+clippy+test+check all pass; taxonomy check 163/163 ✓
<!-- SECTION:NOTES:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
