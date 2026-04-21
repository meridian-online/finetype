---
status: accepted
date-created: 2026-02-28
date-modified: 2026-03-11
---
# 0004. Sense & Sharpen architectural pivot — from tiered CharCNN cascade to column-level transformer routing

## Context and Problem Statement

FineType's tiered CharCNN architecture (34 models, 18 disambiguation rules) reached a structural ceiling:

1. CLDR-enriched retraining (NNFT-157–161) regressed from 116/120 to 107/120, revealing cascading fragility in the tier graph
2. CharCNN capacity ceiling of ≤20 labels per T2 model blocks taxonomy expansion and locale integration
3. 18 disambiguation rules with strict ordering dependencies — each new accuracy fix adds interaction complexity
4. Entity overcall (`full_name` on 3,500+ SOTAB columns) requires column-level context the CharCNN cannot provide
5. The original Burn prototype proved transformers handle larger label spaces and locale-aware classification where CharCNN fails

## Considered Options

- **Option A — Continue with CharCNN + rules (incremental fixes).** Rejected — evidence shows diminishing returns. Each fix risks regressions elsewhere in the tier graph.
- **Option B — Replace entire inference pipeline with a transformer.** Rejected — discards CharCNN's proven strength on format types (datetime, URL, email, etc.).
- **Option C — Sense & Sharpen two-stage pipeline.** Column-level transformer routes to broad category (Sense), then existing flat CharCNN classifies within that category via output masking (Sharpen). Retains CharCNN strengths, adds column-level context.

## Decision Outcome

Chosen option: **Option C — Sense & Sharpen**, because it retains CharCNN's proven format-type accuracy while adding column-level transformer routing that addresses the capacity ceiling, entity overcall, and rule proliferation.

Phased rollout: Phase 0 (taxonomy audit), Phase 1 (Sense model spike), Phase 2 (integration design), Phase 3 (Rust implementation via Candle), Phase 4 (expansion).

### Consequences

- Good, because it breaks the CharCNN capacity ceiling — transformer handles larger label spaces
- Good, because it eliminates 3-5 disambiguation rules by absorbing entity demotion, duration override, and UTC offset override into Sense
- Good, because it unifies header signal and model prediction into a single decision
- Good, because Phase 0 delivers immediate value (cleaner label space) regardless of pivot outcome
- Bad, because two-model coordination replaces tier-graph complexity with a different kind of complexity
- Bad, because training data curation for Sense is a significant effort
- Neutral, because the spike may fail — falling back to current architecture with Phase 0 taxonomy cleanup as consolation
