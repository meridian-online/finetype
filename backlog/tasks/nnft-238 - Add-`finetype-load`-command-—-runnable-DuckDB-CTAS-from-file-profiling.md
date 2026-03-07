---
id: NNFT-238
title: Add `finetype load` command — runnable DuckDB CTAS from file profiling
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 01:18'
updated_date: '2026-03-07 22:04'
labels:
  - cli
  - feature
dependencies: []
references:
  - crates/finetype-cli/src/main.rs
  - labels/definitions_representation.yaml
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The current `schema-for` command outputs a CREATE TABLE where every column is VARCHAR with FineType types as comments. This doesn't help analysts actually load data — they can't copy-paste-run it.

New `finetype load <file>` command that outputs a runnable DuckDB CREATE TABLE AS SELECT statement using the taxonomy's `broad_type` and `transform` fields to produce properly typed columns.

Example output:
```sql
CREATE TABLE titanic AS
SELECT
    CAST(PassengerId AS BIGINT) AS PassengerId,  -- representation.identifier.increment
    Survived,                                     -- representation.boolean.binary
    Name,                                         -- identity.person.full_name
    CAST(Fare AS DOUBLE) AS Fare,                -- representation.numeric.decimal_number
    Embarked                                      -- representation.discrete.categorical
FROM read_csv('titanic.csv', auto_detect=true);
```

Design decisions from interview:
- CTAS over CREATE+INSERT or bare DDL — single runnable statement
- VARCHAR columns use bare column reference (no redundant CAST)
- File path in read_csv() = exactly what the user provided
- Table name = sanitised filename stem (or --table-name override)
- SQL-only output — no -o json/arrow formats
- Trust model predictions as-is — no confidence guards or VARCHAR fallback
- DuckDB-only target (no --target flag)
- Full dotted labels (domain.category.type) as SQL comments
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 finetype load -f titanic.csv outputs a valid, runnable DuckDB CTAS statement
- [x] #2 Columns with non-VARCHAR broad_type use the taxonomy transform expression in the SELECT
- [x] #3 VARCHAR columns appear as bare column references (no CAST(x AS VARCHAR))
- [x] #4 Each column has a -- domain.category.type comment with the full FineType label
- [x] #5 Table name defaults to sanitised filename stem; --table-name flag overrides
- [x] #6 read_csv() uses the exact file path provided by the user
- [x] #7 Command accepts same model/pipeline flags as profile (--model, --sharp-only, --no-header-hint, --sample-size, --delimiter)
- [x] #8 Smoke test: load output for a test CSV can be executed in DuckDB without errors
- [x] #9 Output includes trailing SELECT * FROM {table} LIMIT 10; for immediate preview when piped to DuckDB (--limit N overrides, --limit 0 suppresses)
- [x] #10 Column names in SELECT clause match DuckDB's normalize_names output (lowercase, underscores for spaces/special chars)
- [x] #11 Column names normalised by default (lowercase, spaces→underscores, strip hyphens); --no-normalize-names flag preserves original names. Uses SQL aliases instead of DuckDB normalize_names=true to avoid reserved-word conflicts
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### 1. Add `Load` variant to `Commands` enum (~line 197)
New subcommand with flags:
- `-f, --file` (required) — input CSV path
- `--table-name` — override table name
- `-m, --model` — model directory (default: models/default)
- `--sample-size` — default 100
- `--delimiter` — CSV delimiter
- `--no-header-hint` — disable header hints
- `--model-type` — char-cnn/tiered/transformer
- `--sharp-only` — disable Sense
- `--limit` — preview row count (default: 10, 0 = suppress SELECT)
- `--no-normalize-names` — disable normalize_names in read_csv

### 2. Add `normalize_column_name()` function
Replicate DuckDB's normalize_names exactly:
- Lowercase everything
- Spaces → underscores
- Strip hyphens (not replace!)
- Leading digit → underscore prefix
- Keep existing underscores

### 3. Implement `cmd_load()` function
Reuse the existing column profiling pattern from `cmd_schema_for`:
- Load classifier + taxonomy (same boilerplate)
- Read CSV, profile each column
- Look up `ddl_info()` for each column → get `broad_type` + `transform`
- Build CTAS output:

```sql
CREATE TABLE {table} AS
SELECT
    {transform_expr} AS {col},  -- {label}
    ...
FROM read_csv('{file_path}', auto_detect=true, normalize_names=true);

SELECT * FROM {table} LIMIT 10;
```

Column expression logic:
- If `broad_type == VARCHAR` or `is_generic`: bare column ref (just `col_name`)
- If `broad_type != VARCHAR`: substitute `{col}` in transform with column name → `CAST(col AS BIGINT) AS col`
- If no transform but non-VARCHAR broad_type: `CAST(col AS TYPE) AS col`

When `--no-normalize-names`: omit `normalize_names=true` from read_csv, use original column names

### 4. Wire up in `main()` match
Route `Commands::Load { ... }` → `cmd_load(...)` (same pattern as SchemaFor dispatch ~line 515)

### 5. Smoke test
Create a small test CSV, run `finetype load -f test.csv`, pipe into `duckdb` to verify it executes
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added `finetype load` command — outputs runnable DuckDB CTAS from file profiling.

## What changed
New `finetype load -f <file>` CLI command that profiles a CSV file and generates a complete, pipe-ready DuckDB `CREATE TABLE AS SELECT` statement:

- **Typed columns** use taxonomy `transform` expressions (e.g., `strptime(date, '%Y-%m-%d')::DATE`)
- **VARCHAR/generic columns** appear as bare column references (no redundant CAST)
- **Column name normalization** on by default (lowercase, spaces→underscores, strip hyphens) via SQL aliases; `--no-normalize-names` preserves originals
- **Preview SELECT** appended by default (`SELECT * FROM {table} LIMIT 10`); `--limit N` controls row count, `--limit 0` suppresses
- Uses `all_varchar=true` in `read_csv()` so FineType controls all type casting (prevents DuckDB auto_detect from conflicting with transform expressions)
- Same model/pipeline flags as `profile` (--model, --sharp-only, --no-header-hint, --sample-size, --delimiter)

## Design decision
Used SQL aliases for column normalization instead of DuckDB's `normalize_names=true` parameter. DuckDB additionally prefixes reserved words (name→_name, value→_value) which we can't reliably replicate. Aliases give full control and reserved words work fine in alias position.

## Usage
```bash
finetype load -f data.csv | duckdb              # Profile, create table, show 10 rows
finetype load -f data.csv | duckdb mydb.duckdb  # Persist to database
finetype load -f data.csv --table-name customers --limit 5 | duckdb
finetype load -f data.csv > load.sql            # Save for later
```

## Tests
- `cargo test` — all pass
- `cargo run -- check` — 250/250 taxonomy alignment
- `cargo fmt --check` + `cargo clippy` — clean
- Smoke tested: `finetype load -f covid_timeseries.csv | duckdb` — executes successfully with typed date columns"
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
