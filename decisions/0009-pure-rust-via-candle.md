---
status: accepted
date-created: 2026-03-02
date-modified: 2026-03-11
---
# 0009. Pure Rust via Candle — full Rust ML replacing all Python

## Context and Problem Statement

FineType's ML training pipeline (Sense, Entity, CharCNN, data preparation) was originally implemented in Python/PyTorch. The inference pipeline was already pure Rust. The question: should training remain in Python, or should it be ported to Rust via Candle (Hugging Face's Rust ML framework)?

A feasibility spike (NNFT-182/187) in `crates/finetype-candle-spike/` validated the Candle 0.8 API against FineType's specific requirements.

## Considered Options

- **Path A — Full Rust with Candle.** Port all training to Rust using Candle 0.8. Zero Python dependencies at build and runtime. Single language for the entire stack.
- **Path B — Hybrid (Rust inference + Python training).** Keep Python for training, Rust for inference. Simpler migration but permanent Python dependency for model updates.

## Decision Outcome

Chosen option: **Path A — Full Rust with Candle**, because the feasibility spike validated all critical requirements: cross-attention mechanism, multi-task output, safetensors round-trip, gradient flow, and optimizer step — all passing 10/10 automated tests.

Key technical finding: the initial Session 1 failure (dependency compilation error) was a known Candle 0.8 issue with a simple fix (`half = "2.4"` pin), not a fundamental limitation.

The result is the `finetype-train` crate with 4 binaries (train_sense, train_entity, prepare_sense_data, prepare_model2vec) and zero Python dependencies anywhere in the build or runtime chain.

### Consequences

- Good, because zero Python dependencies simplifies CI, distribution, and contributor onboarding
- Good, because single-language stack means training code can share types, validation, and taxonomy logic with inference
- Good, because Candle supports Metal (macOS) and CUDA acceleration for training
- Bad, because Candle's ecosystem is less mature than PyTorch — fewer examples, less documentation, API changes between versions
- Bad, because Candle CPU training may be 2-3× slower than PyTorch (acceptable for one-time offline training)
- Neutral, because Model2Vec embedding loading was already in Rust; only the training loop needed porting
