---
id: NNFT-144
title: >-
  Discovery: Evaluate whether our benchmarks measure real-world type inference
  quality
status: To Do
assignee:
  - '@nightingale'
created_date: '2026-02-27 00:33'
labels:
  - discovery
  - evaluation
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Our profile eval (70/74 = 94.6%) is a 74-column smoke test that we have been treating as the scoreboard. Our real-world benchmarks — GitTables (47% label) and SOTAB (42% label) — tell a different story.

This discovery asks: do we actually know whether FineType is good at type inference in the real world? And if not, what would a meaningful evaluation look like?

Questions to answer:
- What does the profile eval actually measure vs what we think it measures?
- Are GitTables/SOTAB scores meaningful, or are they dominated by semantic-only types we cannot detect by design?
- What fraction of the SOTAB/GitTables error budget is types we could plausibly detect vs types that need cross-column or external context?
- Is there a smaller, curated real-world benchmark we should build that tests the types analysts actually care about?
- Should we measure something other than label accuracy — e.g. domain accuracy, top-K, analyst satisfaction?

Time-box: 4-6 hours. Output: written finding with data in discovery/ brief.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Written finding answering: what does profile eval actually measure?
- [ ] #2 Breakdown of GitTables/SOTAB error budget by detectability category
- [ ] #3 Recommendation for evaluation approach going forward
- [ ] #4 Discovery brief created at discovery/evaluation-method/BRIEF.md
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
