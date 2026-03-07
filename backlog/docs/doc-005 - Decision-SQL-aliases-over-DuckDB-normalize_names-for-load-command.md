---
id: doc-005
title: 'Decision: SQL aliases over DuckDB normalize_names for load command'
type: other
created_date: '2026-03-07 22:04'
---
# Decision: SQL aliases over DuckDB normalize_names

**Context:** NNFT-238 — `finetype load` command needs to normalize column names for analyst-friendly output.

**Options considered:**

1. **DuckDB `normalize_names=true`** — Let DuckDB handle normalization in `read_csv()`, replicate the logic in our SELECT clause to match.
2. **SQL aliases** — Handle normalization ourselves via `AS` aliases in SELECT, reference original column names (quoted when needed).

**Decision:** Option 2 — SQL aliases.

**Rationale:** DuckDB's `normalize_names` additionally prefixes SQL reserved words with underscore (e.g., `Name`→`_name`, `Value`→`_value`, `Type`→`_type`). Replicating DuckDB's full reserved word list would be fragile and could diverge across DuckDB versions. SQL aliases give us full control over output names, and reserved words work fine in alias position (`SELECT "Name" AS name` is valid SQL).

Additionally, we use `all_varchar=true` instead of `auto_detect=true` in `read_csv()` so FineType controls all type casting via taxonomy transform expressions. Without this, DuckDB auto_detect could cast columns (e.g., dates) before our `strptime` expressions operate on them, causing type mismatches.

**Status:** Implemented in NNFT-238.
