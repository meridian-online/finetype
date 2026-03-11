---
status: accepted
date-created: 2026-03-03
date-modified: 2026-03-11
---
# 0027. Validation-based candidate elimination — >50% failure threshold

## Context and Problem Statement

After CharCNN vote aggregation, the winning type may be incorrect because the model confuses visually similar types. FineType has JSON Schema validation contracts for every type. The question: can validation be used to eliminate impossible candidates before disambiguation rules run?

## Considered Options

- **No validation filtering** — Trust the model vote entirely. Disambiguation rules and header hints handle errors. But some errors are objectively wrong (e.g., predicting `email` for a column of pure integers).
- **Validate winner only** — Check if the winning type's validation passes on sample values. Simple but misses cases where the second-place candidate is valid and the winner isn't.
- **Validate all top candidates, eliminate failures >50%** — After vote aggregation, validate all top candidates against their JSON Schema contracts. Eliminate any candidate where >50% of sample values fail validation. Safety: keep original votes if ALL candidates are eliminated.

## Decision Outcome

Chosen option: **Validate all top candidates with >50% failure elimination**, because it catches objectively impossible predictions without being overly aggressive. The 50% threshold allows for messy real-world data (some values may not conform to the type's schema) while still eliminating gross mismatches.

The safety valve (keep original votes if all eliminated) prevents the validation step from making things worse — if every candidate fails validation, the original model prediction is the best available answer.

Runs before disambiguation rules, providing cleaner input to the rule pipeline.

### Consequences

- Good, because impossible predictions are caught early — e.g., `email` predicted for a column of integers
- Good, because the 50% threshold tolerates messy data (missing values, outliers, format variants)
- Good, because the safety valve prevents validation from degrading predictions when all candidates are poor
- Bad, because validation schemas must be maintained and accurate — a buggy schema could incorrectly eliminate valid types
- Neutral, because this is additive — it can only remove bad candidates, never add new ones
