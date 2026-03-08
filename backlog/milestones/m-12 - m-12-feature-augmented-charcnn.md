---
id: m-12
title: "m-12: Feature-Augmented CharCNN"
---

## Description

Augment CharCNN architecture with parallel deterministic feature vector (~30 features) fused at classifier head. Goal: improve accuracy on 250-type taxonomy beyond 74.1% baseline. Includes per-value features for CNN classification and aggregated column-level features for disambiguation. From ooo interview + seed specification.
