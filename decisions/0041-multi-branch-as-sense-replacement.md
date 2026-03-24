---
status: accepted
date-created: 2026-03-24
date-modified: 2026-03-24
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

The multi-branch model replaces the Sense broad-category classification. Existing disambiguation rules, feature-based guards, and post-processing continue to operate on the model's output. As the model improves (especially with sibling-context training), rules can be progressively retired.

### Consequences

- Good, because the transition is incremental — each improvement in the model allows removing rules
- Good, because regression risk is low — existing post-processing catches model errors
- Good, because evaluation methodology stays the same (190-column profile eval)
- Bad, because maintaining two classification stages adds complexity during the transition
- Neutral, because the multi-branch model's internal architecture is independent of how it integrates with the pipeline
