---
id: NNFT-144
title: >-
  Discovery: Evaluate whether our benchmarks measure real-world type inference
  quality
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-27 00:33'
updated_date: '2026-02-27 01:32'
labels:
  - discovery
  - evaluation
dependencies: []
references:
  - docs/TAXONOMY_COMPARISON.md
  - eval/schema_mapping.csv
  - eval/schema_mapping.yaml
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
- [x] #1 Written finding answering: what does profile eval actually measure?
- [x] #2 Breakdown of GitTables/SOTAB error budget by detectability category
- [x] #3 Recommendation for evaluation approach going forward
- [x] #4 Discovery brief created at discovery/evaluation-method/BRIEF.md
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Audit current benchmarks — profile eval, SOTAB, GitTables error budgets by detectability tier
2. Compute analyst-centric metrics — precision, recall, actionability per type
3. Identify the full_name overcall as dominant quality problem
4. Write discovery brief with data-backed findings and recommendations
5. Present findings and recommendations for approval
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Phase 1-3 complete. Discovery brief written at discovery/evaluation-method/BRIEF.md.
Key findings:
- 62.5% of GT labels are semantic-only — headline label accuracy is structurally misleading
- Datetime precision is 96.6% — the "grudge work saver" metric is excellent
- Email/phone precision near-perfect (100%, 99.7%)
- full_name overcall: only 8.6% of full_name predictions are actually person names
- Recommend: precision per type, actionability eval (TRY_CAST), domain accuracy on format-detectable types
- Stop optimising for SOTAB/GitTables label accuracy — domain accuracy on format-detectable is the right metric

Updated brief with ambitious precision targets and concrete improvement paths:
- >80% precision target for every type FineType claims to detect
- Name-dataset (727K first + 984K last names) for person name validation
- CLDR data for geography validation
- Transformer at T1 VARCHAR as architecture spike
- Overcall eval as new key metric
- Reframed: explain the gap AND close it
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
## NNFT-144: Evaluation methodology discovery

### Key findings

1. **Headline benchmark numbers are structurally misleading** — 62.5% of GT labels are semantic-only (category, description, rating). SOTAB label accuracy is penalised for not being a semantic classifier. Domain accuracy on format-detectable types (72.9%) is the right baseline.

2. **FineType excels at high-value analyst types** — DateTime precision 96.6%, email 100%, phone 99.7%. These are the "saves grudge work" types and they are already excellent.

3. **full_name overcall is the #1 quality problem** — Only 8.6% of full_name predictions are actually person names. 91.4% are music recordings, organisations, restaurants, places, books. This actively misleads analysts.

4. **Geography and URL precision need work** — 46.1% and 32.8% respectively. Below the analyst trust threshold.

### Evaluation approach going forward

- **Precision per type** as the primary metric (>80% target for every claimed type)
- **Overcall eval** measuring false positive rates on full_name, URL, geography
- **Actionability eval** measuring TRY_CAST success rates
- **Domain accuracy on format-detectable** reframed as the real-world metric
- Profile eval stays as regression smoke test

### Improvement paths identified

- Name-dataset (727K first + 984K last, 105 countries) for person name validation
- CLDR geography lists for city/country/region validation
- Transformer/word-embedding classifier at T1 VARCHAR for entity disambiguation
- SOTAB/GitTables as validation corpora (not just scoring)

### Deliverable

- Discovery brief: discovery/evaluation-method/BRIEF.md
- Data analysis: SOTAB error budget by detectability tier, per-type precision, overcall rates
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
