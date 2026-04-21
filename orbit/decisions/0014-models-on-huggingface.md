---
status: accepted
date-created: 2025-12-15
date-modified: 2026-03-11
---
# 0014. Models on HuggingFace — model artifacts stored externally, not in git

## Context and Problem Statement

FineType's trained model artifacts (CharCNN safetensors, Model2Vec embeddings, Sense weights, Entity classifier) are binary files ranging from 400KB to 8MB each. The full model directory is ~30MB. These files change with each retraining cycle and must be available for both CI builds and end-user installations.

The question: where should model artifacts live?

## Considered Options

- **In-repo (git)** — Commit model files directly to the repository. Simple but bloats repo history permanently — each retraining adds ~30MB of binary diffs that git cannot efficiently delta-compress.
- **Git LFS** — Store large files via Git Large File Storage. Better than raw git but adds CI complexity (LFS quota, bandwidth limits, checkout latency).
- **HuggingFace Hub** — Host models on `hughcameron/finetype` HuggingFace repository. CI downloads via script. Models versioned independently of code.

## Decision Outcome

Chosen option: **HuggingFace Hub (`hughcameron/finetype`)**, because it provides purpose-built model hosting with versioning, keeps the git repo lean, and aligns with ML ecosystem conventions. CI downloads models via a script during build.

Models are embedded into the binary at compile time via `include_bytes!` with a build script that expects model files in `models/`. The download script fetches from HuggingFace before build.

### Consequences

- Good, because the git repo stays lean — cloning doesn't download 30MB+ of binary model history
- Good, because model versioning is independent of code versioning — can test new models without code changes
- Good, because HuggingFace provides download metrics and community visibility
- Bad, because CI builds require network access to download models — offline builds need models pre-cached
- Bad, because model-code version coupling must be managed manually (model expects N classes, code must match)
- Neutral, because snapshot learning (auto-snapshot before overwriting) provides local model history regardless of remote hosting
