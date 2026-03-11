---
status: accepted
date-created: 2026-02-13
date-modified: 2026-03-11
---
# 0017. Model embedding via include_bytes! — filesystem-first, embedded-fallback

## Context and Problem Statement

FineType's CLI and DuckDB extension need access to trained model weights (CharCNN, Model2Vec, Sense, Entity, Sibling Context) at runtime. The distribution strategy must support zero-config installation (no manual model downloads) while allowing developers to swap models during development.

The v0.1.0 release shipped broken because model files weren't included in the binary (NNFT-046).

## Considered Options

- **include_bytes! only** — Compile-time embedding. Zero-config but no model swapping without recompile.
- **Download on first run** — Like the whisper DuckDB extension. Adds network dependency, first-run latency, and offline failure mode.
- **Filesystem-first, embedded-fallback** — Check filesystem path first; if not found, fall back to `include_bytes!` embedded weights. Best of both worlds.

## Decision Outcome

Chosen option: **Filesystem-first, embedded-fallback**, because it gives developers model swapping (drop a new safetensors file in `models/`) while end users get zero-config operation. Implemented via `embed-models` Cargo feature flag (default on).

This pattern is used for all model artifacts: CharCNN flat model, Model2Vec embeddings, Sense weights, Entity classifier, and Sibling Context attention. The DuckDB extension also uses `include_bytes!` — at ~5.5MB total embedded models, this is trivially small compared to DuckDB extension norms (34-512MB).

### Consequences

- Good, because zero-config installation — `cargo install` or Homebrew just works
- Good, because developers can iterate on models without recompiling
- Good, because DuckDB extension is fully self-contained and offline-capable
- Bad, because every model change requires a recompile to update embedded weights
- Bad, because the `embed-models` feature flag adds build complexity (CI must download models before build)
- Neutral, because binary size scales with model count — currently ~5.5MB, manageable but grows with each new model artifact
