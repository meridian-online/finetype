---
status: accepted
date-created: 2026-02-24
date-modified: 2026-03-11
---
# 0019. Model2Vec potion-base-4M as semantic embedding model

## Context and Problem Statement

FineType needed a text embedding model for semantic header matching (mapping column names to type labels), Sense model input encoding, and entity classifier features. The model must be embeddable in the binary (<10MB), fast enough for per-column inference, and available as a pure Rust implementation (no Python runtime).

## Considered Options

- **MiniLM-L6-v2** — Full sentence transformer. 91MB FP32. High quality but exceeds size budget by 9×. Requires ONNX runtime or PyTorch.
- **Multilingual MiniLM** — 471MB. Ruled out on size alone.
- **Model2Vec (potion-base-4M)** — Distilled static embeddings from potion-base-4M. 4-15MB depending on vocabulary size. 500× faster than MiniLM. Pure Rust crate available (model2vec-rs). Float16 storage for binary size.
- **TF-IDF** — Traditional approach. No neural network required. But poor semantic matching on short headers.

## Decision Outcome

Chosen option: **Model2Vec (potion-base-4M)**, because it fits the size budget (8MB with float16), has a pure Rust implementation, and provides sufficient semantic quality for header-to-type matching. The 500× speed advantage over MiniLM makes it practical for per-column inference.

Model2Vec embeddings are 128-dimensional. Used for: header hints (max-sim matching against type embeddings), Sense model input (first 50 column values encoded), entity classifier features (mean/std of column value embeddings).

### Consequences

- Good, because pure Rust — no ONNX runtime, no Python, no external dependencies
- Good, because 8MB embedded is well within the 50MB binary budget
- Good, because static embeddings are deterministic and cache-friendly
- Bad, because distilled static embeddings lose contextual nuance — "bank" always has the same embedding regardless of context (financial vs river bank)
- Bad, because vocabulary coverage is limited to potion-base-4M's training data — rare domain-specific headers may embed poorly
- Neutral, because the embedding model can be swapped via the filesystem-first loading pattern (decision-0017) without code changes
