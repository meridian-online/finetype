---
status: accepted
date-created: 2026-02-10
date-modified: 2026-03-11
---
# 0030. Column sample size — 100 values for CharCNN, encode first 50 with Model2Vec

## Context and Problem Statement

Column-level type inference requires sampling values from the column, classifying each, and aggregating via majority vote. Two parameters control this: how many values to sample for CharCNN classification, and how many to encode with Model2Vec for the Sense model.

Larger samples improve vote accuracy but increase latency. Model2Vec encoding is the dominant cost (~50μs per value).

## Considered Options

- **Full column scan** — Classify every value. Accurate but prohibitively slow on large columns (millions of rows).
- **50 values** — Fast but may miss minority patterns in heterogeneous columns.
- **100 values for CharCNN, 50 for Model2Vec** — CharCNN is fast enough for 100 values. Model2Vec encoding is 2× more expensive, so encode only the first 50. Matches the Sense model's training config (`max_values=50`).
- **Adaptive sampling** — Sample size based on column length. More complex, harder to reason about consistency.

## Decision Outcome

Chosen option: **100 for CharCNN, 50 for Model2Vec**, because it balances accuracy and latency. 100 values gives robust majority vote statistics. 50 Model2Vec encodings keep the Sense model input at ~2.5ms (matching training config). The sample size is configurable via `--sample-size` CLI flag.

For the DuckDB extension, the processing chunk (~2048 rows) serves as the natural sample — no explicit sampling is needed.

### Consequences

- Good, because 100 values provides robust vote statistics for disambiguation rules and majority voting
- Good, because encoding only 50 values halves the Model2Vec cost without degrading Sense accuracy (trained on 50)
- Good, because the split is configurable — power users can increase sample size for higher accuracy on ambiguous columns
- Bad, because 100 values may be insufficient for columns with rare formats mixed in (e.g., 1% of values are in a different format)
- Neutral, because the DuckDB extension uses chunk size (~2048) instead of 100, providing higher accuracy at the cost of more computation
