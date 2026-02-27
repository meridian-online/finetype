---
id: decision-003
title: >-
  Entity disambiguation — trained column-level classifier over enum lookups
  (NNFT-150)
date: '2026-02-27 08:53'
status: accepted
---
## Context

full_name is FineType's biggest overcall problem: only 3.7% of 3,500+ SOTAB full_name predictions are actually person names. The rest are cities, organisations, creative works, and generic text. CharCNN can't distinguish at the value level ("London" vs "Johnson") because both are proper nouns with identical character patterns.

Three approaches evaluated:
- **Option A** — Post-vote trained column-level classifier. Only fires when CharCNN vote is ambiguous.
- **Option B** — Replace T2 person node with a transformer trained on column samples.
- **Option C** — Embedding similarity using off-the-shelf Model2Vec (no new training).

Enum-based approaches (NNFT-145) were also considered and rejected prior to the spike.

NNFT-150 spike tested Option C empirically on 2,911 SOTAB entity columns across 4 categories (person 816, place 719, organization 647, creative work 729).

## Decision

**Option A: Build a trained post-vote column-level classifier.**

The spike proved:
- Column-level value distributions carry entity-type signal: 73.6% 4-class accuracy (RF on embeddings + statistical features) vs 25% random baseline.
- But off-the-shelf embeddings are insufficient: silhouette 0.032 (very weak clustering), within-class spread 3-4× larger than between-class distances.
- Even binary person vs non-person only achieves 84.9% with 21% false positives.

Option C rejected: 73% accuracy with massive class overlap is not production quality.
Option B premature: doesn't justify changing the value-level inference contract until we prove Option A works.
Enum approaches rejected: only cover geography (closed set), don't generalise to orgs/creative works (open set), create maintenance burden. Against the "design for the future" pillar.

## Consequences

- Next task: train a column-level entity classifier on SOTAB data (~3k labelled entity columns).
- Model fires post-vote, only when CharCNN vote is ambiguous between person/entity name types.
- Adds a new model artifact to build, train, and embed.
- Does NOT change the existing tiered inference pipeline or value-level contract.
- If accuracy proves strong enough (>85% on 4-class), may promote to Option B (T2-level integration) in the future.
- Training data source: SOTAB validation split (2,911 columns), expandable with GitTables and synthetic generation.

