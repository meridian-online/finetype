---
status: accepted
date-created: 2026-02-18
date-modified: 2026-03-11
---
# 0018. DuckDB chunk-aware column classification — scalar-over-LIST with chunk context

## Context and Problem Statement

FineType's DuckDB extension needed to perform column-level type inference (majority vote, disambiguation, header hints) from within DuckDB's scalar function interface. DuckDB processes data in ~2048-row chunks. The challenge: scalar functions operate per-row, but column-level inference needs to see multiple values.

## Considered Options

- **Raw C API aggregate function** — 5 unsafe callbacks, manual heap state management. True aggregate semantics but the DuckDB Rust crate (v1.4.4) has no aggregate trait — requires unsafe FFI.
- **Scalar-over-LIST with GROUP BY** — User passes `list(column)` via GROUP BY. Clean Rust API but requires user to write GROUP BY queries.
- **Chunk-aware scalar** — `finetype(col)` automatically uses the processing chunk (~2048 rows) as a column sample for disambiguation. Appears as a scalar function but internally performs column-level inference on the chunk. LIST overload remains for explicit GROUP BY control.

## Decision Outcome

Chosen option: **Chunk-aware scalar**, because it provides column-level accuracy from a simple `SELECT finetype(col) FROM table` without requiring GROUP BY. The ~2048-row chunk provides a statistically representative sample for vote aggregation and disambiguation.

The unified `finetype()` function handles both scalar (chunk-aware) and LIST (explicit grouping) via overloading. `finetype_detail()` provides the same with full JSON output.

### Consequences

- Good, because the simplest possible SQL interface — users write `finetype(col)` and get column-level accuracy
- Good, because ~2048 rows is sufficient for majority vote and disambiguation (similar to CLI's 100-value sample)
- Bad, because chunk boundaries are non-deterministic — different chunk sizes could theoretically yield different results for borderline columns
- Bad, because the scalar function returns the same type for every row in a chunk, which is semantically unusual for a scalar
- Neutral, because the LIST overload provides an escape hatch for users who need explicit grouping control
