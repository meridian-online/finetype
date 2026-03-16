---
status: accepted
date-created: 2026-03-15
date-modified: 2026-03-15
---
# 0036. Delegate column name normalization to DuckDB's normalize_names

## Context and Problem Statement

The `finetype load` command normalizes column names (lowercase, spaces→underscores, strip hyphens) by computing normalized names in Rust and emitting `AS alias` clauses in the generated SQL. This duplicates DuckDB's built-in `normalize_names=true` parameter on `read_csv()`, and our implementation doesn't fully match DuckDB's behaviour (e.g. no camelCase→snake_case splitting).

More importantly, embedding normalization in SQL aliases hides it from the user. If `normalize_names=true` appears in the `read_csv()` call instead, the user can see it and remove it to preserve original names.

## Considered Options

- **Option A:** Add `normalize_names=true` to the `read_csv()` call and remove our alias-based normalization
- **Option B:** Keep our custom normalization and improve it to match DuckDB's behaviour

## Decision Outcome

Chosen option: "Option A", because it gives the user visible control over normalization, avoids duplicating DuckDB's logic, and produces correct camelCase→snake_case splitting for free.

The `--no-normalize-names` flag controls whether `normalize_names=true` appears in the generated `read_csv()` call. Column references in the SELECT always use the original header names (quoted when necessary); DuckDB normalizes the output column names at read time.

### Consequences

- Good, because users can see and toggle normalization by editing the `read_csv()` parameter
- Good, because DuckDB's normalization handles camelCase→snake_case, which our function did not
- Good, because less custom code to maintain
- Bad, because DuckDB prefixes reserved words (e.g. `name`→`_name`) which we previously avoided — acceptable trade-off since analysts can adjust
