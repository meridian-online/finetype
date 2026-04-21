---
status: accepted
date-created: 2026-02-10
date-modified: 2026-03-11
---
# 0022. Training data — 1500 synthetic samples per type with deterministic generators

## Context and Problem Statement

CharCNN models need labeled training data for all types in the taxonomy (currently 250). Real-world labeled data at this scale doesn't exist — no public dataset covers all 250 FineType types with clean labels. The question: how to generate sufficient training data, and how many samples per type?

## Considered Options

- **Real-world data (manual labeling)** — Highest quality but prohibitively expensive at 250 types. Noisy labels from automated extraction degrade training.
- **LLM-labeled data** — NNFT-269 explored this with Qwen3 8B on 5,359 columns: 97% valid labels but only 20% agreement with FineType predictions. LLM defaults to container types for ambiguous inputs. Not yet production-quality.
- **Synthetic generation (deterministic)** — Purpose-built generators per type that produce realistic samples using locale data, format patterns, and domain knowledge. Seed-based determinism for reproducibility.
- **Mixed synthetic + real** — Combine synthetic with real-world samples. Not yet implemented.

## Decision Outcome

Chosen option: **Synthetic generation at 1500 samples per type**, because it provides full taxonomy coverage with controlled quality and reproducibility. Each type has a dedicated generator in `labels/definitions_*.yaml` that produces realistic samples using CLDR locale data, format patterns, and domain-specific rules.

Sample count evolution: 500/type (v2) → 1000/type (v9-v13) → 1500/type (v14-v15). The increase to 1500 improved training accuracy from ~83% to 86.6% on 250 classes. Deterministic generation with `--seed 42` ensures reproducible training runs.

Two types are excluded from generation: `password` and `plain_text` — these are residual types resolved via disambiguation, not character patterns.

### Consequences

- Good, because full taxonomy coverage — every type has training data from day one of its addition
- Good, because deterministic generation with `--seed N` makes training runs reproducible
- Good, because generator quality is directly testable (`finetype check` validates alignment)
- Bad, because synthetic data has distribution mismatch with real-world data — feature fusion regressed due to this (decision-0011)
- Bad, because generator quality is the ceiling on model quality — garbage generators produce garbage models
- Neutral, because LLM-labeled real data (NNFT-269) is a future path to augment synthetic data, pending label quality improvements
