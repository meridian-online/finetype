---
status: accepted
date-created: 2026-03-20
date-modified: 2026-03-20
---
# 0039. Use All Adjudicated Distillation Rows, Not Just Agreement

## Context and Problem Statement

The Tier 2 benchmark spec initially required distilled samples to come from agreement rows only (where the blind Claude classification matched FineType's classification). This would limit the benchmark to 92 types from 24K rows. However, 80 types exist *only* in disagreement rows — cases where FineType consistently misclassifies (username, first_name, currency amounts, date formats). These are exactly the types the benchmark needs to measure improvement on.

## Considered Options

- **Agreement rows only** — highest label confidence (both systems agreed), but biased toward cases the current model already handles. 92 types, 24K rows.
- **All adjudicated rows** — every row where the agent produced a final_label with reasoning, regardless of agreement. 172 types, 85K rows. Includes the error cases that motivate retraining.
- **High + medium confidence only** — exclude the 5.4% low-confidence rows. Marginal quality gain, negligible type coverage change.

## Decision Outcome

Chosen option: "All adjudicated rows", because the adjudication process itself is the quality gate — not the agreement status. The blind-first protocol (Claude classifies blind → FineType classifies independently → agent adjudicates with reasoning) produces a reviewed final_label for every row. Agreement-only filtering would systematically exclude the types that retraining most needs to improve, defeating the purpose of the benchmark.

### Consequences

- Good, because the benchmark covers 172 distilled types instead of 92
- Good, because disagreement rows (80 types, 61K rows) represent FineType's actual weaknesses — the retraining signal we need
- Good, because the benchmark can measure agreement-vs-disagreement accuracy separately, showing where the model improves
- Bad, because some adjudicated labels may be wrong (Claude isn't perfect) — but this is acceptable for an algorithmic benchmark that will be rebuilt as more data arrives
- Bad, because disagreement rows have lower label certainty than agreement rows — mitigated by tracking source_agreement in the benchmark for stratified analysis
