---
id: NNFT-119
title: 'Discovery: specialise Model2Vec embeddings for column name classification'
status: To Do
assignee: []
created_date: '2026-02-24 08:36'
labels:
  - discovery
  - accuracy
  - semantic-hints
dependencies: []
references:
  - scripts/prepare_model2vec.py
  - crates/finetype-model/src/semantic.rs
  - models/model2vec/
documentation:
  - discovery/model2vec-specialisation/BRIEF.md
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Spike to evaluate how to hone the Model2Vec semantic hint system beyond the current "plug in potion-base-4M" integration. See `discovery/model2vec-specialisation/BRIEF.md`.

**Context:** The Model2Vec integration (NNFT-110) improved profile eval from 55/74 to 68/74 by using semantic column name similarity to override generic predictions. But we used an off-the-shelf general-purpose model with minimal synonym coverage (~1 synonym per type on average) and a conservative 0.70 threshold tuned on just 30 test names.

**Driving example:** `people_directory.salary` is misclassified as postal_code@0.91. "Salary" is a semantically clear column name that the embedding model should recognise as price/number-related — but it likely falls below threshold because no type has "salary" in its synonym list, and the general-purpose vocabulary may not embed analytics terms well.

**Three investigation areas:**
1. **Synonym expansion** — Expand from ~1 to 5-10 synonyms per type using real-world column name mining (GitTables, SOTAB, Kaggle) and domain-specific aliases
2. **Custom vocabulary distillation** — Distill from MiniLM using analytics/database column naming vocabulary instead of general English
3. **Threshold refinement** — Per-domain thresholds, confidence calibration, or relative ranking instead of flat 0.70

**Time budget:** 4 hours
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Measure current similarity distribution for all 209 profile eval column names — quantify how many correct matches fall below 0.70
- [ ] #2 Evaluate synonym expansion: add 5-10 synonyms per type for top-20 most misclassified types and measure impact on similarity scores
- [ ] #3 Evaluate custom distillation: distill with analytics-domain vocabulary and compare embedding quality against potion-base-4M
- [ ] #4 Assess false positive risk: test lowered thresholds against a larger set of generic column names
- [ ] #5 Written finding in discovery/model2vec-specialisation/ with concrete numbers and recommendation
<!-- AC:END -->
