---
status: accepted
date-created: 2026-02-27
date-modified: 2026-03-11
---
# 0021. Precision-per-type as primary evaluation metric

## Context and Problem Statement

FineType was initially evaluated by headline label accuracy on external benchmarks (GitTables 1M, SOTAB CTA). These metrics were structurally misleading: ~62.5% of types in these benchmarks are semantic-only (e.g., "company name", "movie title") that no format-based classifier can detect. Optimizing for SOTAB label accuracy meant optimizing for types FineType fundamentally cannot classify.

The question: what evaluation methodology actually measures whether FineType is useful to analysts?

## Considered Options

- **Headline label accuracy on external benchmarks** — SOTAB/GitTables label accuracy. Industry-standard comparison. But structurally misleading for a format-based classifier: v0.1.8 → v0.3.0 was a lateral move (GitTables -1.3pp, SOTAB +1.1pp) despite significant engineering effort.
- **Precision per predicted type** — For each type FineType actually predicts, what fraction are correct? Traffic-light thresholds: 🟢≥95%, 🟡80-95%, 🔴<80%. Focus on types FineType claims to detect, not types it cannot.
- **Profile eval on curated datasets** — Label accuracy on hand-curated CSV datasets with known ground truth. Measures real-world utility directly.

## Decision Outcome

Chosen option: **Precision-per-type + profile eval as primary metrics**, with SOTAB/GitTables as secondary benchmarks scoped to format-detectable types only.

- **Profile eval** (30 datasets, 186 columns, 293 manifest entries): 96.8% label accuracy, 98.4% domain accuracy — the acceptance test for any pipeline change
- **Precision per predicted type**: traffic-light dashboard per type, ensuring no type has unacceptable false-positive rates
- **Actionability** (99.9%): verifies that predicted types have working transformation contracts (TRY_CAST/TRY_STRPTIME succeed)
- **SOTAB/GitTables**: reported for reference but scoped to format-detectable types only

### Consequences

- Good, because development effort focuses on types FineType can actually detect — no more chasing semantic-only types
- Good, because precision-per-type catches "attractor" types (types that absorb unrelated predictions) early
- Good, because actionability eval verifies the full contract — classification → transformation → successful cast
- Bad, because profile eval datasets must be curated manually — each new type needs test coverage
- Bad, because profile eval can overfit to curated datasets — SOTAB/GitTables remain necessary as out-of-distribution checks
- Neutral, because the three metrics (profile, precision, actionability) sometimes conflict — a change can improve one while regressing another
