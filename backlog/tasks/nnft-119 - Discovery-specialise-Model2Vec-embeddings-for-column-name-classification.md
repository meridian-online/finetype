---
id: NNFT-119
title: 'Discovery: specialise Model2Vec embeddings for column name classification'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-24 08:36'
updated_date: '2026-02-25 01:29'
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
priority: high
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
- [x] #1 Measure current similarity distribution for all 209 profile eval column names — quantify how many correct matches fall below 0.70
- [x] #2 Evaluate synonym expansion: add 5-10 synonyms per type for top-20 most misclassified types and measure impact on similarity scores
- [ ] #3 Evaluate custom distillation: distill with analytics-domain vocabulary and compare embedding quality against potion-base-4M
- [x] #4 Assess false positive risk: test lowered thresholds against a larger set of generic column names
- [x] #5 Written finding in discovery/model2vec-specialisation/ with concrete numbers and recommendation
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Discovery spike — 4 hour budget

1. **Measure current baseline** (AC #1)
   - Write Python script to compute cosine similarity for all 209 profile eval column names
   - Classify each as: correct match above threshold, correct match below threshold, wrong match above threshold, no match
   - Identify the gap: how many correct matches are we losing below 0.70?

2. **Evaluate synonym expansion** (AC #2)
   - Expand synonyms for top-20 most misclassified types
   - Re-compute type embeddings with expanded synonyms
   - Measure improvement in similarity scores and correct match rate

3. **Evaluate custom distillation** (AC #3)
   - Build analytics-domain vocabulary list
   - Distill from MiniLM-L6-v2 with custom vocab
   - Compare embedding quality vs potion-base-4M

4. **Assess false positive risk** (AC #4)
   - Test lowered thresholds against expanded generic name set
   - Find optimal threshold balancing recall vs precision

5. **Write finding** (AC #5)
   - Concrete numbers, recommendation, next steps
   - Save to discovery/model2vec-specialisation/
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #1 baseline measurement complete.

Key findings from 206 profile eval columns:
- 96/206 (46.6%) correct matches above 0.70 threshold (active TPs)
- 50/206 (24.3%) correct matches below threshold (lost opportunities)
- 6/206 (2.9%) wrong matches above threshold (false positives)
- 54/206 (26.2%) correctly rejected

Recoverable opportunities by bucket:
- [0.65, 0.70): 12 columns — easy wins with threshold adjustment
- [0.60, 0.65): 7 columns — tracking_url, order_date, event_date, alpha-2/3, rating, unix_ms
- [0.50, 0.60): 5 columns — description, locale, Ticket, semantic_version
- Below 0.50: 26 columns — need vocabulary improvement, not just threshold

Threshold sweep shows:
- 0.65: 108 TP, 8 FP (93.1% precision, 74.0% recall)
- 0.70: 96 TP, 6 FP (94.1% precision, 65.8% recall) ← current
- Lowering to 0.65 gains +12 TP for +2 FP

Driving example: salary → decimal_number at 0.477. Already a synonym in prepare_model2vec.py but far below threshold — vocabulary gap.

AC #2: Synonym expansion tested — 244 new synonyms across 19 types. +6 TP at 0.70 but 29 regressions from centroid dilution. Net negative at lower thresholds. Key insight: mean-pooling many diverse synonyms produces generic centroids.

AC #3: Custom distillation deferred — requires torch (not installed). Vocabulary list prepared (213 terms). Estimated 30 min to test when torch available.

AC #4: False positive assessment on 163 generic names. At 0.65 threshold, only 1 genuine FP (data → form_data at 0.687). xml/csv/json/text above threshold are actually correct matches, not FPs.

AC #5: Written finding in discovery/model2vec-specialisation/FINDING.md with concrete numbers and recommendation.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Discovery spike complete: Model2Vec specialisation for column name classification.

Findings:
- Baseline: 96/206 (46.6%) correct matches above 0.70, with 50 lost opportunities below threshold
- Threshold adjustment (0.70 → 0.65): +12 TP for +2 FP — the highest-value, lowest-risk change
- Synonym expansion: +6 TP at 0.70 but causes 29 regressions from centroid dilution (mean-pooling too many diverse synonyms produces generic centroids)
- False positive risk: Only 1 genuine FP on 163-name generic set at 0.65 (data → form_data)
- Custom distillation: Deferred (requires torch), vocabulary list prepared

Recommendation:
1. Immediate: Lower threshold from 0.70 to 0.65 (single constant change)
2. Short-term: Add targeted synonyms for ~5 specific types (timezone, postal_code, url, http_status_code, mime_type)
3. Medium-term: Implement max-sim matching instead of mean-pooled centroids to enable aggressive synonym expansion without dilution
4. Long-term: Custom distillation with analytics-domain vocabulary

Deliverables:
- discovery/model2vec-specialisation/analyse_similarity.py — baseline similarity measurement
- discovery/model2vec-specialisation/evaluate_synonym_expansion.py — expansion impact analysis
- discovery/model2vec-specialisation/evaluate_distillation.py — distillation + FP assessment
- discovery/model2vec-specialisation/FINDING.md — written finding with data and recommendation
<!-- SECTION:FINAL_SUMMARY:END -->
