---
name: finetype-pipeline
description: >
  Use when profiling, typing, or materialising CSV/TSV data. Guides the full FineType
  pipeline: profile → schema → validate (typed materialisation built in). Ensures
  agents complete all steps rather than stopping after profile.
---

# FineType Pipeline — Profile to Typed Table

FineType detects 250 semantic types in text data and maps each to a DuckDB expression
guaranteed to succeed. This skill guides you through the **complete pipeline** — do not
stop after profiling.

## Pipeline Overview

```
profile → schema → validate (--db --table)
   ↓         ↓         ↓
  What    Capture   Quality gate AND typed
  types   as JSON   DuckDB table + reject
  exist   Schema    sidecar — single pass
```

**Every step matters.** Profiling alone tells you what types exist but does not catch
bad rows or generate loadable SQL. Always complete the pipeline.

## Step 1: Profile the Dataset

Detect the semantic type and DuckDB storage type for every column:

```bash
finetype profile -f data.csv
```

**Read the output carefully:**
- **TYPE** — the three-part semantic label (e.g., `identity.person.email`)
- **BROAD** — the recommended DuckDB storage type (VARCHAR, BIGINT, TIMESTAMP, DECIMAL)
- **CONF** — column confidence. Below 90% means some values don't match the dominant type — this is a **data quality signal**, not a FineType error
- **Sense hints** (in brackets) — which detection strategy fired

**Options you may need:**
- `--delimiter ';'` or `--delimiter '\t'` for non-CSV files
- `--sample-size 500` for large files (default samples 100 rows per column)
- `-o json` for machine-readable output
- `-v` for verbose pipeline tracing (shows Sense, mask, hint decisions)

**Do not stop here.** Profile is step 1 of 4.

## Step 2: Generate a Schema

Capture the profile results as a JSON Schema with validation rules
(table-mode export migrated from the retired `schema` verb to
`profile -o json-schema` in v0.6.19 — MADR 0070):

```bash
finetype profile -f data.csv -o json-schema > data.schema.json
```

This writes a sidecar file `data.schema.json` containing:
- JSON Schema validation rules (patterns, min/max length) for each column
- `x-finetype-label` — the semantic type key
- `x-finetype-broad-type` — the DuckDB storage type
- `x-finetype-transform` — the DuckDB cast expression
- `x-finetype-confidence` — the column confidence score
- `required` array — columns with no nulls observed

**Options:**
- `--stdout` to print to stdout instead of writing a file
- `--stats` to include observed data statistics (min, max, cardinality, null rate)

**Save the schema** — it becomes the contract between raw data and typed tables.

## Step 3: Validate the Data (Quality Gate)

Run every row through the schema as a quality gate. Default mode is
**check-only** — no files written, exit code communicates pass/fail:

```bash
finetype validate data.csv data.schema.json
```

Exit codes:
- `0` — no rejects (all rows pass)
- `1` — rejects present (engine SEMANTIC_TYPE or transform-failed)
- `2` — error (malformed schema, file unreadable, missing `duckdb`, etc.)

`--lenient` forces exit 0 regardless of rejects (useful in inspection
contexts where you want the report but not a non-zero exit).

**Read the validation report:**
- **Grade** (A–F) based on overall pass rate
- **Per-column breakdown** showing valid, invalid, and null counts
- A column at 91.7% valid means ~8% of values fail the pattern — investigate those

**Decision point — what to do with invalid rows:**

| Situation | Action |
|-----------|--------|
| Grade A/B (>80% valid) | Materialise (Step 4) — invalid rows land in the reject sidecar, not the user table |
| Grade C/D (50–80%) | Investigate the reject sidecar — schema may be too strict or data needs cleaning |
| Grade F (<50%) | Do not materialise. Check if the delimiter or encoding is wrong, or if the data needs preprocessing |

## Step 4: Materialise into DuckDB

Pass `--db <out.db> --table <name>` to `validate` to materialise valid rows
into a typed DuckDB table — per-column transforms applied via TRY-wrapped
projection — alongside the `finetype_reject_errors` sidecar. Single pass,
single CTAS, single validation engine.

```bash
finetype validate data.csv data.schema.json --db mydb.db --table data
duckdb mydb.db -c "SELECT * FROM data LIMIT 10;"
duckdb mydb.db -c "SELECT column_name, error_type, constraint_failed, expected_type FROM finetype_reject_errors;"
```

Reject ontology:
- `error_type='SEMANTIC_TYPE'` — engine validation failure (pattern, enum, range, …)
- `error_type='TRANSFORM_FAILED'` (`constraint_failed='transform'`) — cell
  passed validation but failed the typed cast (e.g. `2024-02-30` matches a
  date pattern but `strptime` rejects it). `error_message` carries the
  literal `transform_failed: <transform-expr>`.

Staging-NULL → typed-NULL is **not** a transform failure. Empty cells
surface as DuckDB NULL with no reject row.

ENUM emission is dropped — low-cardinality columns retain the schema's
`duckdb_type` (typically VARCHAR). If you need enum semantics, declare
them explicitly in the JSON Schema's `enum` keyword.

`finetype validate --db --table` requires `duckdb` on PATH. Exit codes:
0 no rejects / 1 rejects (engine + transform) / 2 error. `--lenient`
forces 0. `--append` reuses an existing `.db` and increments `scan_id`.

## Complete Pipeline Example

```bash
# 1. Profile — understand what you have
finetype profile -f contacts.csv

# 2. Schema — capture as a contract
finetype profile -f contacts.csv -o json-schema > contacts.schema.json

# 3. Validate + materialise — quality gate AND typed table in one pass
finetype validate contacts.csv contacts.schema.json --db contacts.db --table contacts
```

## Quick Path (Skip Schema)

If you trust the data quality and just need typed columns fast, run the
schema and validate steps back-to-back. There's no separate "load"
verb — `validate --db --table` is the single typed-output path:

```bash
finetype profile -f data.csv -o json-schema > schema.json
finetype validate data.csv schema.json --db mydb.db --table data
```

`finetype load …` was removed in v0.6.19 (MADR 0071) — it now errors
via clap's unknown-subcommand handler with exit 2.

## Exploring Individual Values

Use `infer` to classify a single value when you need to understand how FineType sees it:

```bash
finetype infer -i "alice@example.com" --confidence
# → identity.person.email  1.0000

finetype infer -i "not-an-email" --confidence
# → identity.person.username  0.9963
```

This is useful for debugging why a column has low confidence — check the outlier values.

## Exploring the Type System

Use `taxonomy` to browse available types:

```bash
# All types
finetype taxonomy

# Filter by domain
finetype taxonomy -d identity

# Filter by category
finetype taxonomy -c person

# Full export with descriptions and validation rules
finetype taxonomy --full -o json
```

## Key Principles

1. **Profile is step 1, not the destination.** Always continue to schema → validate → load.
2. **Confidence below 90% is a signal.** Investigate the outlier values with `finetype infer -i "suspect_value" --confidence`. If the *type itself* is wrong (not just dirty values), edit the schema manually before validating.
3. **Validate before loading.** The quality gate catches issues that will cause silent failures in DuckDB.
4. **Use the valid CSV for loading.** `data.csv.valid.csv` is guaranteed to cast cleanly.
5. **Schema is the contract.** Save it alongside your data — it documents what the data should look like.
