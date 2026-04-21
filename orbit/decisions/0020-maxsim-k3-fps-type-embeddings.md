---
status: accepted
date-created: 2026-02-25
date-modified: 2026-03-11
---
# 0020. Max-sim K=3 FPS for type embeddings — replacing mean-pool centroids

## Context and Problem Statement

FineType's semantic header hints use Model2Vec to match column names against pre-computed type embeddings. The original approach used mean-pooled single-centroid embeddings (one 128-dim vector per type, averaged from synonym lists like ["email", "email_address", "e_mail", "electronic_mail"]).

Synonym expansion (NNFT-122) revealed a centroid dilution problem: adding more synonyms moved the centroid away from all of them, reducing match quality. "email_address" and "electronic_mail" have different embedding locations — their mean is near neither.

## Considered Options

- **Mean-pool centroids (K=1)** — One vector per type. Simple but causes centroid dilution with diverse synonyms.
- **K=3 FPS (Farthest Point Sampling)** — Select 3 representative embeddings per type that maximize coverage of the synonym space. Match via max-sim (highest similarity across all K embeddings). Zero-padded for types with <K synonyms.
- **K=5 or K=10** — More representatives. Diminishing returns vs compute cost.
- **Learned projection head** — Train a small network to project headers into type space. More powerful but adds training complexity.

## Decision Outcome

Chosen option: **K=3 FPS with max-sim matching**, because it eliminates centroid dilution while keeping the embedding artifact small (N_types × 3 × 128 dimensions). Farthest Point Sampling selects the 3 most diverse synonyms per type, ensuring coverage of the synonym space.

The 0.65 similarity threshold for hint activation was calibrated empirically. Match quality: K=3 FPS at 0.65 threshold outperforms K=1 centroid at any threshold on the profile eval benchmark.

### Consequences

- Good, because synonym expansion no longer degrades match quality — adding synonyms can only help
- Good, because max-sim is cheap (3 dot products per type, vectorized)
- Good, because zero-padding handles types with <3 synonyms gracefully
- Bad, because the embedding artifact is 3× larger than single-centroid (750 × 128 → 750 × 3 × 128)
- Neutral, because the threshold (0.65) may need recalibration if the embedding model changes
