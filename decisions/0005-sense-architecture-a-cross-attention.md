---
status: accepted
date-created: 2026-02-28
date-modified: 2026-03-11
---
# 0005. Sense model architecture — Architecture A (cross-attention over Model2Vec)

## Context and Problem Statement

Phase 1 of the Sense & Sharpen pivot (decision-004). Two Sense model architectures were prototyped in Python/PyTorch and evaluated on SOTAB + synthetic data (NNFT-163). The Sense model's job is column-level broad category classification (6 categories + 4 entity subtypes) using sampled values + column header.

## Considered Options

- **Architecture A — Cross-attention over Model2Vec embeddings.** Header embedding attends over value embeddings via cross-attention, producing a weighted summary. Dual classification heads (broad + entity) from concatenated [attention_output, value_mean, value_std]. ~5K parameters, 0.15ms/column inference.
- **Architecture B — Transformer encoder.** Standard 2-layer transformer encoder over value + header embeddings with [CLS] token pooling. ~85K parameters, 3.55ms/column inference.

## Decision Outcome

Chosen option: **Architecture A (cross-attention)**, because it achieved +1.6pp higher accuracy (88.5% vs 86.9% broad), was 23.7× faster (0.15ms vs 3.55ms), had 17× fewer parameters, and presented a simpler Candle porting surface.

Architecture A's advantage comes from treating the header as a query over value evidence — the header "asks" the values what category they belong to. This aligns with how column inference actually works: the header carries strong prior signal, and the values provide confirming evidence.

### Consequences

- Good, because 0.15ms/column inference fits within DuckDB extension latency budget
- Good, because ~5K parameters means Sense adds only ~1.4MB to model artifacts (shared Model2Vec weights)
- Good, because the simpler architecture reduces Candle porting effort (Phase 3)
- Bad, because Architecture B's self-attention could theoretically learn richer inter-value relationships — but the empirical evidence shows this doesn't help for broad category classification
- Neutral, because both architectures depend on Model2Vec embedding quality — switching embedding models would require retraining either way
