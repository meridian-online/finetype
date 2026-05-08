# Interview: Baseline Misclassification Diagnosis

**Date:** 2026-03-23
**Interviewer:** Nightingale

---

## Context

The production Sense→Sharpen pipeline (char-cnn-v14-250) scores 148/190 (78%) on label accuracy against the expanded eval set. Previously reported as ~98% on a smaller 174-column set. 42 columns are misclassified. Before we can judge whether multi-branch adds value, we need to understand these failures: are they real model weaknesses, bad ground truth, or eval infrastructure issues?

## Interview Q&A

### Q1: Goal
**Q:** What's the primary goal of this diagnosis — is it to fix the baseline accuracy (ship improvements to users), or to clean up the eval set so multi-branch comparisons are trustworthy?
**A:** Fix baseline accuracy. Triage the 42 failures, fix actionable ones (rules, generators, GT labels), ship real accuracy improvement to users.

### Q2: Bad GT handling
**Q:** How should we handle ground truth labels that look wrong (e.g., a column labelled 'full_name' that's actually job titles)?
**A:** Fix GT labels. Correct schema_mapping.csv when the ground truth is clearly wrong — reduces noise in the eval set.

### Q3: Fix scope
**Q:** What kinds of pipeline fixes are in scope? Some failures need new disambiguation rules or generator changes — others might need Sense category routing changes or new post-processing layers.
**A:** Triage first, decide per-bucket. Categorise all 42 failures first, then decide which fix categories to pursue based on effort vs impact.

### Q4: Delivery
**Q:** Should the triage itself be a standalone deliverable, or do you want triage and fixes shipped in the same PR?
**A:** Triage PR first, fixes later. Ship a triage PR with the categorised failure list and findings. Then a separate PR for each fix category we decide to pursue.

### Q5: Methodology
**Q:** For the triage methodology — should I run the eval fresh and extract failures programmatically, or work from existing eval results?
**A:** Fresh baseline eval run + DuckDB analysis. Re-run the production baseline, capture full JSON output, use DuckDB to query for failure clustering and pattern analysis.

---

## Summary

### Goal
Fix baseline accuracy on the 190-column expanded eval set by triaging all 42 misclassifications, correcting bad ground truth labels, and identifying actionable pipeline improvements.

### Constraints
- Triage before committing to fixes — categorise all failures first
- Fix GT labels when they're clearly wrong
- Fresh eval run with DuckDB-based analysis (not stale results)
- Triage PR ships first, fix PRs follow per-bucket

### Success Criteria
- All 42 failures categorised into buckets (bad GT, missing rule, generator gap, genuinely hard)
- Each bucket has effort vs impact assessment
- Corrected GT labels in schema_mapping.csv
- Findings committed as a triage PR
- Clear recommendation on which fix buckets to pursue

### Open Questions
- What's the target accuracy after fixes? (Depends on how many are bad GT vs real failures)
- Should fixes that touch disambiguation rules be tested against the full eval set before merging? (Likely yes — regression risk)
