---
status: accepted
date-created: 2026-02-10
date-modified: 2026-03-11
---
# 0016. Flat CharCNN for DuckDB extension — not tiered

## Context and Problem Statement

FineType has two model architectures: flat (single model, 1 pass) and tiered (T0→T1→T2, 34 models, 3 passes). The CLI uses tiered for its higher interpretability and modular retraining. The DuckDB extension needed to choose between them.

Benchmarking (NNFT-016) showed flat CharCNN at 91.97% accuracy vs tiered at 90.00%, with flat being 1.7× faster (single forward pass vs cascade of 3).

## Considered Options

- **Flat CharCNN** — Single model, single forward pass. Higher accuracy (91.97%), faster inference, simpler embedding (1 model file).
- **Tiered model** — 34 models in a cascade. More interpretable (domain → category → type routing), modular retraining per tier. But slower (3 passes) and slightly less accurate due to error cascading between tiers.

## Decision Outcome

Chosen option: **Flat CharCNN for DuckDB extension**, because it is both more accurate and faster than tiered for this use case. DuckDB extension priorities are throughput and accuracy over interpretability. The CLI continues to support both architectures.

After the Sense→Sharpen pivot (decision-004), the CLI also shifted to flat CharCNN with Sense-guided output masking, making this the default architecture everywhere.

### Consequences

- Good, because single-model embedding simplifies the DuckDB extension build (one safetensors file, not 34)
- Good, because single forward pass maximizes throughput in SQL query context
- Good, because flat CharCNN actually beats tiered on eval — error cascading between tiers costs ~2pp
- Neutral, because the tiered architecture remains available via `--tiered` flag but is no longer the default anywhere
