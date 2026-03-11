---
status: accepted
date-created: 2026-02-27
date-modified: 2026-03-11
---
# 0003. Entity disambiguation — trained column-level classifier over enum lookups

## Context and Problem Statement

`full_name` is FineType's biggest overcall problem: only 3.7% of 3,500+ SOTAB full_name predictions are actually person names. The rest are cities, organisations, creative works, and generic text. CharCNN can't distinguish at the value level ("London" vs "Johnson") because both are proper nouns with identical character patterns.

NNFT-150 spike tested embedding similarity (Option C) empirically on 2,911 SOTAB entity columns across 4 categories (person 816, place 719, organization 647, creative work 729).

## Considered Options

- **Option A — Post-vote trained column-level classifier.** Only fires when CharCNN vote is ambiguous. Requires training a new model on SOTAB data.
- **Option B — Replace T2 person node with a transformer.** Higher capacity but changes the value-level inference contract.
- **Option C — Embedding similarity using off-the-shelf Model2Vec.** No new training required. Spike result: 73.6% 4-class accuracy, but silhouette 0.032 (very weak clustering), within-class spread 3-4× larger than between-class distances.
- **Enum lookups (NNFT-145)** — Rejected prior to spike. Only covers geography (closed set), doesn't generalise to orgs/creative works (open set).

## Decision Outcome

Chosen option: **Option A — trained post-vote column-level classifier**, because the spike proved column-level value distributions carry entity-type signal (73.6% accuracy on embeddings + statistical features), but off-the-shelf embeddings are insufficient for production quality. A trained Deep Sets MLP (300→4 classes) fires post-vote only when CharCNN predicts `full_name`, demoting to `entity_name` when non-person confidence >0.6.

### Consequences

- Good, because entity overcall is dramatically reduced without changing the value-level inference pipeline
- Good, because the classifier is composable — fires post-vote, doesn't modify CharCNN
- Bad, because it adds a new model artifact to build, train, and embed
- Bad, because training data (SOTAB) is limited to ~3K columns
- Neutral, because if accuracy proves strong enough (>85% on 4-class), may promote to T2-level integration in the future
