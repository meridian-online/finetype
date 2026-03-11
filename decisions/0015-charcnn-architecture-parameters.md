---
status: accepted
date-created: 2026-02-10
date-modified: 2026-03-11
---
# 0015. CharCNN architecture parameters — small model with SGD

## Context and Problem Statement

FineType needed a character-level neural network for type classification from raw string values. The model must be small enough to embed in a CLI binary and DuckDB extension (<50MB total), fast enough for per-value inference, and accurate enough to distinguish 250+ types from character patterns alone.

## Considered Options

- **Small architecture** — char embedding (vocab=97, dim=32) → conv1d (64 filters, kernel=3) → global max pool → FC (128 hidden) → N-class output. SGD optimizer. ~390KB at 250 classes.
- **Medium architecture** — wider filters (128/256), multiple kernel sizes (3,5,7), deeper FC layers. ~2-5MB.
- **Large architecture** — multi-scale CNN with attention, or byte-level transformer (ByT5/CANINE). 10-500MB.

## Decision Outcome

Chosen option: **Small architecture with SGD**, because it achieves 86.6% accuracy at 250 classes while staying under 500KB. The architecture has remained stable through 15 model versions (v1 through v15), proving sufficient for format-type discrimination.

Key parameters: vocab size 97 (printable ASCII + padding), embedding dim 32, 64 conv filters with kernel size 3, global max pool, single hidden layer of 128 units, SGD with learning rate 0.01. Two size variants exist: "small" (32/64/128) for production and "large" (64/128/256) for experiments.

### Consequences

- Good, because <500KB model size enables embedding in both CLI and DuckDB extension binaries
- Good, because single-pass inference is fast enough for per-value classification in column batches
- Good, because SGD's simplicity avoids Adam's momentum state — smaller memory footprint during training
- Bad, because the small architecture has a capacity ceiling — it cannot learn fine-grained distinctions between visually similar types (git_sha vs hash, hs_code vs decimal_number), which is why post-vote rules (F1–F5) exist
- Neutral, because the "large" variant exists for experiments but has never outperformed "small" on eval metrics
