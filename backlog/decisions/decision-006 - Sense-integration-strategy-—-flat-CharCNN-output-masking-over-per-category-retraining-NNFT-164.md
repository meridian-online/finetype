---
id: decision-006
title: >-
  Sense integration strategy — flat CharCNN + output masking over per-category
  retraining (NNFT-164)
date: '2026-02-28 23:07'
status: proposed
---
## Context

Phase 2 of the Sense & Sharpen pivot (decision-004). Phases 0-1 complete: taxonomy audit
(NNFT-162, 171→163 types) and model spike (NNFT-163, Architecture A wins per decision-005:
88.5% broad accuracy, 78% entity subtype, 3.6ms/column). This decision covers the
integration design choices for Phase 3 (Rust implementation).

## Decision

Three key choices:

**1. Flat CharCNN + output masking** — Use the existing flat char-cnn-v7 (169 classes) with
Sense-guided output masking instead of retraining 6 category-specific models. After Sense
predicts a broad category, mask CharCNN votes to eligible types within that category.
Alternative rejected: per-category retraining adds infrastructure complexity and risks
regressions. Start simple; retrain in Phase 4 if accuracy insufficient.

**2. Sample 100, encode 50** — Keep sample_size=100 for CharCNN. First 50 values encoded
with Model2Vec for Sense. Matches spike training config (max_values=50), keeps encoding
at ~2.5ms. Alternative: encode all 100. Rejected — doubled cost for marginal gain.

**3. Sense absorbs 6 behaviours** — Entity demotion (Rule 18), header hints (semantic +
hardcoded), geography protection, entity demotion guard, is_generic gating, measurement
disambiguation all subsumed. Header is a direct Sense input; entity subtyping replaces
EntityClassifier. 12 value-level disambiguation rules retained.

**Additional:** Shared Model2VecResources (+1.4MB Sense weights only), backward-compatible
fallback when Sense absent, DuckDB extension unchanged.

## Consequences

- Phase 3 is 8 tasks (NNFT-165 through NNFT-172), estimated 7-9 implementation days
- Net speedup: ~73ms → ~25ms per column (2.9x) by replacing tiered 34-model cascade with flat CharCNN
- Memory increase: +1.4MB for Sense model weights (shared Model2Vec avoids triple loading)
- Header hint system (~200 lines of Rust) becomes dead code when Sense is active, retained for fallback
- EntityClassifier retained for fallback path but no longer needed when Sense is active
- Design document: discovery/architectural-pivot/PHASE2_DESIGN.md
