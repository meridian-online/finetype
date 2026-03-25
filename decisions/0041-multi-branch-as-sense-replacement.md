---
status: accepted
date-created: 2026-03-24
date-modified: 2026-03-25
---
# 0041. Multi-branch model as Sense replacement within the existing pipeline

## Context and Problem Statement

FineType has two inference paths: (1) Sense→Sharpen — Sense classifier predicts a broad category, masks the flat CharCNN vote, then disambiguation rules and header hints post-process the result (188/190 accuracy); (2) Multi-branch (Sherlock-style) — a single 4-branch model (char + embed + stats + header) that predicts directly (140/190 raw, no post-processing).

The question is whether multi-branch should replace the entire pipeline or serve as a component within it. This decision has been made multiple times in conversation but never recorded.

## Considered Options

- **Multi-branch replaces everything** — Single model becomes the sole inference path. No Sense, no tiered models, no disambiguation rules. Clean architecture but needs ≥188/190 to justify the switch.
- **Multi-branch as better Sense within the pipeline** — Use multi-branch as a smarter first-stage classifier, then apply existing disambiguation/validation as post-processing. Combines model strength with proven rules.
- **Parallel paths, best-of-breed** — Keep both pipelines, compare per-column, take the best. Research mode.

## Decision Outcome

Chosen option: "Multi-branch as better Sense within the pipeline", because:

1. The existing disambiguation rules and post-processing are proven and debuggable. Discarding them requires the model to independently match 188/190 — an unrealistic near-term target.
2. Multi-branch at 140/190 raw is already a strong single-model result. Used as a Sense replacement, it provides better broad-category predictions than the current Sense classifier while the downstream pipeline handles edge cases.
3. This is consistent with the "strength through simplification" philosophy (decision 0038) — reduce rules over time as the model improves, but don't discard working infrastructure prematurely.

### Pipeline architecture (clarified 2026-03-25)

The multi-branch model replaces **both** the Sense classifier and the CharCNN value-level voting. It is the new "Sense" layer — a column-level model that produces a type label in a single forward pass. This is the primary speed advantage: one forward pass per column instead of ~100 CharCNN value inferences.

The "Sharpen" layer becomes the lightweight post-processing that already exists:
- Feature-based disambiguation rules (F1–F6)
- Entity demotion (confidence-based)
- Locale detection via `validation_by_locale` patterns
- Header-based semantic matching (Model2Vec)

These are cheap operations (no neural inference) and recover accuracy the model misses. As the model improves, rules are progressively retired per decision 0038.

```
Old:   Sense → CharCNN voting → mask → disambiguation → output
New:   Multi-branch → lightweight Sharpen → output
       (1 forward pass)  (cheap rules, no neural inference)
```

### Consequences

- Good, because the architecture is simpler — one model + one post-processing layer, same as before but with better components
- Good, because inference is significantly faster — single column-level forward pass replaces ~100 value-level forward passes per column
- Good, because each improvement in the model allows retiring Sharpen rules (decision 0038)
- Good, because evaluation methodology stays the same (190-column profile eval)
- Neutral, because CharCNN and Sense are fully retired — no fallback path, no legacy mode
