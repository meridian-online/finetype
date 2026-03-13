---
status: accepted
date-created: 2026-03-13
date-modified: 2026-03-13
---
# 0035. Add permanent tracing instrumentation to Sense→Sharpen pipeline

## Context and Problem Statement

The column classification pipeline (Sense→Sharpen) makes several sequential decisions — Sense category prediction, vote masking, disambiguation rules, header hints, feature rules — but provides no visibility into intermediate state. When misclassifications occur (e.g., earthquake `horizontalError` classified as `boolean.initials` instead of `decimal_number`), debugging requires reading source code and guessing which decision went wrong. This pipeline has been a recurring debugging target across multiple milestones.

## Considered Options

- **Option A: One-off diagnostic** — Add temporary print statements, debug the current bug, remove them.
- **Option B: Permanent `tracing::debug!`** — Add structured tracing at all 6 pipeline decision points, activated via `RUST_LOG`.
- **Option C: Permanent tracing + `--verbose` CLI flag** — Option B plus a discoverable CLI flag that enables debug tracing without requiring `RUST_LOG` knowledge.

## Decision Outcome

Chosen option: "Option C — Permanent tracing + `--verbose` flag", because:

1. The pipeline has been debugged manually across multiple milestones (sibling context, F1–F6 rules, header hints). Permanent tracing prevents repeating this work.
2. `tracing::debug!` is zero-cost when not enabled (compiled out in release without subscriber).
3. A `--verbose` flag makes the tracing discoverable for users, not just developers.

### Trace Points

1. Sense prediction — category, confidence, entity subtype
2. Vote aggregation — raw CharCNN votes before masking (top 5 with counts)
3. Mask application — votes surviving mask, votes removed, safety valve decision
4. Header hints — which hint fired, what it changed
5. Feature rules F1–F6 — which rule fired, what it changed
6. Final result — label, confidence, full disambiguation trail

### Consequences

- Good, because future pipeline bugs can be diagnosed from trace output without reading source
- Good, because `--verbose` is user-facing and aids bug reports
- Bad, because adds ~50 `tracing::debug!` calls to column.rs (code noise)
- Neutral, because zero runtime cost when tracing is not enabled
