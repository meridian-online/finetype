---
id: NNFT-150
title: >-
  Spike: Test whether column-level value embeddings separate entity types
  (person vs place vs org)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-27 08:08'
updated_date: '2026-02-27 08:55'
labels:
  - discovery
  - disambiguation
  - entity_name
  - model
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The full_name overcall is FineType's biggest accuracy problem — only 3.7% of 3,500+ full_name predictions in SOTAB are actually person names. The rest are cities, organisations, creative works, and generic text. CharCNN can't distinguish these at the value level ("London" vs "Johnson"), so we need column-level signal.

This spike tests whether Model2Vec value embeddings, aggregated per column, carry enough signal to separate entity types. The result determines which disambiguation approach to build:

- If embeddings cluster well → Option C (embedding similarity, no new model training)
- If weak/noisy separation → Option A or B (trained column-level classifier, placement TBD)
- If no signal → back to the drawing board

**Data source:** SOTAB columns with Schema.org type labels (Person, Place, Organization, CreativeWork — ~16k labelled columns available).

**Method:** Embed actual cell values (not column names) with Model2Vec (potion-base-4M), aggregate per column (mean), measure inter-class vs intra-class distance and kNN classification accuracy on held-out split.

**Time-box:** 3-4 hours. Output: finding with numbers and recommendation.

**Supersedes NNFT-145** — enum-based city disambiguation was rejected as architecturally unsound (maintenance burden, only covers geography, doesn't generalise).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Embedding separation measured on SOTAB columns for at least 3 entity types (Person, Place, Organization)
- [x] #2 Quantitative result: kNN accuracy or silhouette score on column-level embeddings
- [x] #3 Recommendation: Option A (trained column classifier), B (T2 replacement), or C (embedding similarity) with supporting data
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Ran two spike scripts on 2,911 SOTAB entity columns (Person 816, Place 719, Organization 647, Creative Work 729).

Key results:
- Silhouette score: 0.032-0.037 (very weak clustering)
- Best kNN (embeddings only, k=11): 73.2% accuracy on 4 classes
- Best overall (RF on embeddings+features): 73.6%
- Statistical features alone (RF): 63.8%
- Binary person vs non-person: 84.9% but with 21% FP rate
- Organization is hardest (63% F1), creative works easiest (82% F1)
- Within-class spread 3-4x larger than between-class centroid distances

Conclusion: Option C insufficient (73% ≠ production quality). Option A recommended — trained column-level classifier using combined embedding + statistical features. Signal exists but needs a purpose-trained model to extract properly.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Tested whether Model2Vec value embeddings (potion-base-4M, 128-dim), aggregated per column, can separate entity types in SOTAB data. 2,911 columns across 4 categories: person (816), place (719), organization (647), creative work (729).

Result: Signal exists but is insufficient for production use.
- Best accuracy: 73.6% (Random Forest on embeddings + 20 statistical features) vs 25% random baseline
- Silhouette: 0.032 — very weak clustering, massive class overlap
- Organization hardest (63% F1), creative works easiest (82% F1)
- Binary person vs non-person: 84.9% accuracy but 21% false positive rate

Recommendation: **Option A** — build a trained post-vote column-level classifier. The column distribution signal is real (73% from off-the-shelf tools), but a purpose-trained model is needed to reach production quality. Option C (embedding similarity alone) rejected. Option B (T2 replacement) premature.

Finding written: discovery/entity-disambiguation/FINDING.md
Spike scripts: discovery/entity-disambiguation/embedding_spike.py, embedding_spike_extended.py
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
