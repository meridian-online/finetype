---
id: decision-004
title: >-
  Sense & Sharpen architectural pivot — from tiered CharCNN cascade to
  column-level transformer routing
date: '2026-02-28 03:42'
status: accepted
---
## Context

FineType's tiered CharCNN architecture (34 models, 18 disambiguation rules) has reached a structural ceiling:

1. **CLDR-enriched retraining** (NNFT-157–161) regressed from 116/120 to 107/120, revealing cascading fragility in the tier graph — not isolated data issues.
2. **CharCNN capacity ceiling** of ≤20 labels per T2 model (NNFT-126) blocks taxonomy expansion and locale integration.
3. **18 disambiguation rules** with strict ordering dependencies — each new accuracy fix adds interaction complexity.
4. **v0.1.8 → v0.3.0 was a lateral move** at macro level (GitTables regressed 1.3pp, SOTAB gained 1.1pp).
5. **Entity overcall** (full_name on 3,500+ SOTAB columns) requires column-level context the CharCNN cannot provide.
6. **Burn prototype** proved transformers handle larger label spaces and locale-aware classification where CharCNN fails.

Three options were evaluated:

- **Option A: Continue with CharCNN + rules** (incremental fixes). Rejected — evidence shows diminishing returns.
- **Option B: Replace entire inference pipeline.** Rejected — discards CharCNN's proven strength on format types.
- **Option C: Sense & Sharpen two-stage pipeline.** Selected — retains CharCNN for format types, adds column-level transformer for semantic routing.

## Decision

Adopt the **Sense & Sharpen** two-stage pipeline:

**Stage 1 — Sense (Transformer, column-level):** Samples 50 values per column, embeds them with column name as input, produces broad semantic category + entity subtype. Replaces T0→T1 routing, entity classifier bolt-on, and header hint overrides.

**Stage 2 — Sharpen (CharCNN + validation, value-level):** Retained CharCNN models, scoped by Sense output. Validation schemas continue as-is.

**Stage 3 — Confirm (rules, reduced set):** 12-14 surviving rules (down from 18), operating in narrower scope with fewer interaction dependencies.

### Phased rollout

| Phase | Scope | Duration |
|-------|-------|----------|
| Phase 0 | Taxonomy audit — collapse ~7-10 niche types, re-baseline eval (NNFT-162) | 1-2 days |
| Phase 1 | Sense model spike — Python/PyTorch prototype, two architectures (NNFT-163) | 1 week |
| Phase 2 | Integration design — Sense→Sharpen interface, surviving rules mapping | 2-3 days |
| Phase 3 | Rust implementation via Candle | 1-2 weeks |
| Phase 4 | Expansion and polish — new types, CLDR locale integration | ongoing |
| Future | DuckDB extension redesign — separate workstream | TBD |

### Design constraints

- Sense model must be **embeddable** (`include_bytes!`, trait-based interfaces) for future DuckDB extension adoption.
- Column name is a **Sense input**, not a separate post-classification override — unifies header signal and model prediction.
- Phase 0 is independent and delivers value regardless of pivot outcome.

## Consequences

### Positive

- Breaks the CharCNN capacity ceiling — transformer handles larger label spaces
- Eliminates 3-5 disambiguation rules by absorbing entity demotion (Rule 18), duration override (Rule 14), and UTC offset override (Rule 17) into Sense
- Unifies header signal and model prediction into a single decision — collapses header hint overrides, geography protection, and entity demotion guard
- Enables future locale integration via column-level transformer
- Phase 0 delivers immediate value (cleaner label space) regardless of pivot outcome

### Negative

- Two-model coordination replaces tier-graph complexity with a different kind of complexity
- Training data curation for Sense stage is the longest-pole item (~2 days of Phase 1)
- Spike may fail — fall back to current architecture with Phase 0 taxonomy cleanup as consolation
- Realistic rule reduction is 18→12-14, not 18→5-8 as originally proposed

### Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Transformer latency | Column sampling (50 values, not all values); Architecture A (attention over Model2Vec) is fast |
| Training data insufficient | Start with 2,911 SOTAB entity columns + synthetic category labels; expand incrementally |
| Format regression | CharCNN retained for all format types; Sense only routes, doesn't replace |
| DuckDB extension locked out | Embeddable Sense interface (`include_bytes!`, trait-based); extension redesign is a future workstream |
| Spike fails | Keep current architecture; Phase 0 taxonomy cleanup still delivers value |

### References

- Discovery brief: `discovery/architectural-pivot/BRIEF.md`
- Phase 0 task: NNFT-162
- Phase 1 task: NNFT-163
- Prior decisions: decision-002 (locale detection), decision-003 (entity classifier)
- Evidence: NNFT-126 (CharCNN capacity), NNFT-150 (entity embedding spike), NNFT-157–161 (CLDR regression)
