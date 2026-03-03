---
id: NNFT-189
title: 'v0.5.1 Documentation: README and CHANGELOG update'
status: Done
assignee:
  - '@docs-writer'
created_date: '2026-03-03 06:31'
updated_date: '2026-03-03 07:34'
labels:
  - documentation
dependencies: []
references:
  - CLAUDE.md
  - README.md
  - CHANGELOG.md
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Update README.md and CHANGELOG.md to reflect v0.5.1 milestones. The codebase has had major changes since the last documentation update: Pure Rust training (no Python), Sense→Sharpen pipeline as default, 164-type taxonomy, accuracy improvements to 98.3% label accuracy, DuckDB extension, evaluation infrastructure.

README should reflect:
- Current architecture (Sense→Sharpen pipeline)
- Updated type counts (164 types, 7 domains)
- Pure Rust stack (no Python dependencies)
- Current evaluation baselines
- Updated CLI commands and examples

CHANGELOG should cover v0.5.1 milestones:
- Taxonomy v0.5.1 (NNFT-177/178/179/180)
- Pure Rust training (NNFT-185)
- Python cleanup (NNFT-186)
- Model retrain (NNFT-181)
- Accuracy improvements (NNFT-188)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 README.md updated with current architecture, type counts, and evaluation baselines
- [x] #2 CHANGELOG.md updated with v0.5.1 milestone entries
- [x] #3 No code changes — documentation only
- [x] #4 Examples in README are accurate and runnable
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Update README.md headline stats and features section
   - Type count: 163 → 164 across 6 → 7 domains (finance is new)
   - Accuracy: 96.7% → 98.3% profile eval (116/120 → 117/119)
   - Test count: update once cargo test finishes
   - Feature bullets: update type count, accuracy, model names

2. Update README.md taxonomy table
   - Add finance domain (16 types: IBAN, SWIFT/BIC, currency amounts, etc.)
   - Fix all domain counts to match current: container 12, datetime 45, finance 16, geography 16, identity 20, representation 31, technology 24

3. Update README.md architecture section
   - Mermaid diagram is outdated (shows only tiered cascade, not Sense→Sharpen)
   - Update pipeline description to document Sense→Sharpen as default
   - Update model accuracy table: CharCNN v7 → v9, class count 163 → 164
   - Update crate table: 7 → 8 crates (add finetype-build-tools)
   - Update repo structure tree (type counts, crate count)

4. Update README.md Performance section
   - Profile eval accuracy: 96.7% → 98.3% label (117/119), 99.2% domain
   - Update model table with current model names and class counts

5. Update README.md CLI section
   - Verify 10 commands listed vs what actually exists
   - Remove or correct any commands that have changed

6. Update README.md Known Limitations
   - Update locale support text if needed (type count change)

7. Add CHANGELOG.md v0.5.1 section
   - Accuracy: NNFT-188 improvements (108/119 → 117/119)
   - Added: Pure Rust training (NNFT-185), finance domain types
   - Changed: Taxonomy 163 → 164, model retrain (NNFT-181)
   - Removed: Python training scripts (NNFT-186)

8. Verify examples by running finetype infer on sample values

9. Run cargo test + finetype check to confirm no regressions (DoD)
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- Team lead approved plan with corrections:
  - Keep 7 crates (not 8 — finetype-build-tools not merged yet)
  - Domain accuracy: 100% (119/119) per eval report
  - Label accuracy: 98.3% (117/119) confirmed
- Starting edits now

CLAUDE.md already accurate — no update needed (verified: 164 types, 98.3% label, 100% domain all match)

No decision record needed — straightforward documentation update, no approach choices
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Updated README.md and CHANGELOG.md to reflect v0.5.1 milestones.

## README.md changes
- Headline: 163→164 types, 6→7 domains (finance domain added), 96.7%→98.3% label accuracy (117/119)
- Features: updated type count, accuracy, clarified pure Rust (no Python at all)
- Taxonomy table: added finance domain (16 types), updated all domain counts to match current
- Model accuracy table: updated to CharCNN v9 (164 classes), Sense→Sharpen as default
- Architecture section: replaced legacy tiered-only mermaid diagram with Sense→Sharpen pipeline flow. Updated pipeline stages table with Model2Vec, Sense, masked aggregation, entity demotion, validation elimination
- "Why Tiered CharCNNs?" renamed to "Why Sense→Sharpen?" explaining the two-stage approach
- CLI section: removed hidden developer commands (train, eval, eval-gittables), added schema command
- Development section: removed train/eval commands from examples, removed hard-coded test count
- Updated all stale 163-type references to 164
- Crate table: updated roles and dependencies (jsonschema, validate command)

## CHANGELOG.md changes
- Added v0.5.1 section with Accuracy, Added, Changed, Removed subsections
- Documented: NNFT-188 accuracy improvements, finance domain (NNFT-177/178), identifier types (NNFT-179/180), pure Rust training (NNFT-185), model retrain (NNFT-181), Python removal (NNFT-186)

## Verification
- All 260 core+model tests pass, taxonomy check 164/164
- CLI examples verified against current binary (v0.5.1)
- No code changes — documentation only
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
