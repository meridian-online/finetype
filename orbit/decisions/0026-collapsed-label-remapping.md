---
status: accepted
date-created: 2026-02-28
date-modified: 2026-03-11
---
# 0026. Collapsed label remapping for backward compatibility

## Context and Problem Statement

When types are removed from the taxonomy (decision-0023) or renamed (decision-0025), existing trained models still predict the old label strings. Rather than immediately retraining the model after every taxonomy change, the pipeline needs a mechanism to map old predictions to their replacement types.

## Considered Options

- **Immediate model retrain** — Retrain after every taxonomy change. Guarantees model and taxonomy are always in sync. But training takes hours and risks regressions.
- **Alias resolution in taxonomy YAML** — YAML `aliases` field maps old names to current names. Clean but only handles renames, not removals or type merges.
- **Code-level remapping** — `remap_collapsed_label()` function in `column.rs` that maps old model predictions to replacement types at vote aggregation time. Handles renames, removals, and merges.

## Decision Outcome

Chosen option: **Code-level remapping via `remap_collapsed_label()`**, because it decouples taxonomy evolution from model retraining. 8 types are currently remapped (e.g., removed types → their replacement, renamed types → new names). The function runs at vote aggregation and semantic hint output.

This enables incremental taxonomy changes — types can be added, removed, or renamed without blocking on a retrain cycle. The model is retrained periodically (major milestones) rather than after every YAML change.

### Consequences

- Good, because taxonomy changes ship immediately without waiting for model retraining
- Good, because git history shows exactly which labels are remapped and why
- Bad, because the remapping list grows with each taxonomy change until the next retrain clears it
- Bad, because remapped predictions may have different confidence characteristics than natively predicted types
- Neutral, because YAML `aliases` are used in parallel for user-facing backward compatibility (schema lookups, taxonomy queries)
