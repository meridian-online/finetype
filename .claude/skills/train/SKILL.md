---
name: train
description: Train a FineType CharCNN model with hardware auto-detection (Metal/CUDA/CPU)
user-invocable: true
---

# Train a FineType Model

Run from the finetype repo root (`~/github/noon-org/finetype/`).

## Quick Start

```bash
# Quick test (CPU, ~5 min)
./scripts/train.sh --samples 100 --size small --epochs 2

# Standard training
./scripts/train.sh --samples 1000 --size small --epochs 10

# Large model on M1 Mac (Metal auto-detected)
./scripts/train.sh --samples 5000 --size large --epochs 15 --seed 42
```

## Architecture Presets

| Preset | embed_dim | num_filters | hidden_dim |
|--------|-----------|-------------|------------|
| small  | 32        | 64          | 128        |
| medium | 64        | 128         | 256        |
| large  | 128       | 256         | 512        |

Override individual params: `--embed-dim 64 --num-filters 128 --hidden-dim 256`

## Hardware Detection

- **macOS** -> Metal GPU (Apple Silicon)
- **Linux + NVIDIA** -> CUDA GPU
- **Otherwise** -> CPU fallback

The script passes `--features metal` or `--features cuda` to Cargo automatically.

## What It Does

1. **Generate** training data (`finetype generate --samples N`)
2. **Build** CLI with correct hardware features
3. **Train** CharCNN with progress display (epoch/loss/accuracy/ETA)

Output goes to `models/char-cnn-vN/` (auto-incremented). Training log saved alongside.

## All Flags

```
--samples N         Samples per type (default: 1000)
--size PRESET       small|medium|large (default: small)
--epochs N          Training epochs (default: 10)
--seed N            Random seed (default: 42)
--embed-dim N       Override embedding dimension
--num-filters N     Override CNN filters
--hidden-dim N      Override hidden layer dimension
--model-name NAME   Output dir name (default: auto char-cnn-vN)
--data FILE         Use existing NDJSON (skip generation)
```

## After Training

```bash
./scripts/eval.sh --model models/char-cnn-vN    # Evaluate
./scripts/package.sh models/char-cnn-vN          # Package for distribution
```
