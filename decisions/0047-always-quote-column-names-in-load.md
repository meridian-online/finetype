---
status: accepted
date-created: 2026-04-14
date-modified: 2026-04-14
---
# 0047. Always quote column names in load command, remove normalize_names

## Context and Problem Statement

The `finetype load` command generates DuckDB CTAS SQL with `normalize_names=true` in the `read_csv()` call (decision 0036). DuckDB's normalization renames reserved-word columns before the SELECT clause sees them — e.g., `name` becomes `_name`, `type` becomes `_type`. Our SELECT expressions still reference the original column names, producing a `Binder Error: Referenced column "name" not found` when the SQL is piped to DuckDB.

This was observed with the `airports.csv` dataset which has columns `name`, `type`, and `source` — all DuckDB reserved words.

## Considered Options

- **Option A:** Remove `normalize_names=true` from `read_csv()`, always double-quote column names in SELECT expressions
- **Option B:** Keep `normalize_names=true` and replicate DuckDB's normalization logic (including reserved word detection) in Rust
- **Option C:** Use a CTE/subquery to separate normalization from column references

## Decision Outcome

Chosen option: "Option A", because it eliminates the mismatch at the source. Double-quoting column names is SQL-standard, handles all edge cases (reserved words, spaces, special characters), and avoids tracking DuckDB's reserved word list across versions.

The `--no-normalize-names` CLI flag is retained but hidden for backward compatibility — it is now a no-op since `normalize_names=true` is never emitted.

This supersedes decision 0036.

### Consequences

- Good, because `finetype load -f file.csv | duckdb` works for all column names including reserved words
- Good, because no dependency on DuckDB's internal reserved word list
- Good, because double-quoted identifiers are SQL-standard and unambiguous
- Bad, because output column names preserve original casing (no automatic camelCase→snake_case) — acceptable trade-off since the SQL is meant for analysts who can adjust
