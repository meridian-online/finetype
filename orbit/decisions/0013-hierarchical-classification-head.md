---
status: accepted
date-created: 2026-03-09
date-modified: 2026-03-11
---
# 0013. Hierarchical classification head — tree softmax over flat output

## Context and Problem Statement

FineType's CharCNN used a flat 250-class softmax output layer. Literature review (NNFT-267, informed by the architecture challenge in `specs/sense-architecture-challenge/`) showed that hierarchical classification is low-hanging fruit for taxonomies with natural tree structure — FineType's `domain.category.type` label format directly encodes a 3-level hierarchy (7 domains → 43 categories → 250 types).

The question: does replacing the flat softmax with a tree softmax improve accuracy, and does it fit within the model size and latency constraints?

## Considered Options

- **Flat softmax (status quo)** — Single 250-class output layer. Simple, proven at 86.6% accuracy (v14). No structural prior over the label space.
- **Tree softmax (hierarchical head)** — Per-node linear layers at each tree level: domain head (7 classes), per-domain category heads (43 total), per-category leaf heads (250 total). Product probabilities: p(type) = p(domain) × p(cat|domain) × p(leaf|cat). Multi-level CE loss with weights λ=0.2/0.3/0.5 (domain/category/leaf).

## Decision Outcome

Chosen option: **Tree softmax (hierarchical head)**, because it achieved 84.2% type accuracy, 90.9% domain accuracy, and 96.5% category accuracy on CharCNN v15-250 — maintaining the 180/186 profile eval baseline while providing graceful degradation (when leaf classification fails, domain/category are often still correct).

Key design choices:
- CharCnn runs in dual mode: `new()` for flat (default, backward compatible), `new_hierarchical()` for tree
- Degenerate categories (only 1 type) skip the leaf head — 39 non-degenerate leaf heads, 4 skipped
- `HierarchyMap` derived from label strings at model load time, not hardcoded
- `--hierarchical` CLI flag to select mode

### Consequences

- Good, because domain-level accuracy (90.9%) provides useful signal even when leaf-type classification fails
- Good, because <100K additional parameters — negligible size increase
- Good, because backward compatible — flat mode remains the default, existing models work unchanged
- Bad, because leaf-type accuracy (84.2%) is slightly lower than flat baseline (86.6%) — the hierarchical constraint trades leaf precision for structural coherence
- Neutral, because the hierarchical head is architecturally available but not yet the default — requires more training data or fine-tuning to surpass flat accuracy
