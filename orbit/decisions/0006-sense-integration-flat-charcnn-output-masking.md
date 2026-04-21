---
status: accepted
date-created: 2026-02-28
date-modified: 2026-03-11
---
# 0006. Sense integration strategy — flat CharCNN + output masking over per-category retraining

## Context and Problem Statement

Phase 2 of the Sense & Sharpen pivot (decision-004). Phases 0-1 complete: taxonomy audit (NNFT-162, 171→163 types) and model spike (NNFT-163, Architecture A wins per decision-005: 88.5% broad accuracy, 78% entity subtype, 3.6ms/column). This decision covers the integration design choices for Phase 3 (Rust implementation).

Three key design questions: (1) retrain per-category CharCNN models or mask the existing flat model? (2) how many values to sample and encode? (3) which existing behaviours does Sense absorb?

## Considered Options

- **Flat CharCNN + output masking** — Use existing flat char-cnn-v7 (169 classes) with Sense-guided category masking. After Sense predicts a broad category, mask CharCNN votes to eligible types within that category.
- **Per-category retraining** — Train 6 category-specific CharCNN models, each handling only its category's types. Higher potential accuracy but massive infrastructure complexity.

## Decision Outcome

Chosen option: **Flat CharCNN + output masking**, because it avoids retraining risk and infrastructure complexity. Additional choices:

- **Sample 100, encode 50** — Keep sample_size=100 for CharCNN. First 50 values encoded with Model2Vec for Sense (matches spike training config, keeps encoding at ~2.5ms).
- **Sense absorbs 6 behaviours** — Entity demotion (Rule 18), header hints (semantic + hardcoded), geography protection, entity demotion guard, is_generic gating, measurement disambiguation. Header is a direct Sense input; entity subtyping replaces EntityClassifier.

### Consequences

- Good, because net speedup ~73ms → ~25ms per column (2.9x) by replacing tiered 34-model cascade with flat CharCNN
- Good, because existing flat model is proven — zero retraining risk
- Good, because shared Model2VecResources avoids triple loading (+1.4MB Sense weights only)
- Bad, because header hint system (~200 lines of Rust) becomes dead code when Sense is active (retained for fallback)
- Neutral, because backward-compatible fallback path exists when Sense model is absent
