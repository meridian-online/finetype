---
name: finetype-pipeline
description: >
  Use when profiling, typing, or loading CSV/TSV data. Guides the full FineType
  pipeline: profile → schema → validate → load. Ensures agents complete all steps
  rather than stopping after profile.
---

# FineType Pipeline — Profile to Typed Table

FineType detects 250 semantic types in text data and maps each to a DuckDB expression
guaranteed to succeed. This skill guides you through the **complete pipeline** — do not
stop after profiling.

## Pipeline Overview

```
profile → schema → validate → load
   ↓         ↓         ↓         ↓
  What    Capture   Quality    Typed
  types   as JSON   gate       DuckDB
  exist   Schema    (split     table
                    valid/
                    invalid)
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

Capture the profile results as a JSON Schema with validation rules:

```bash
finetype schema data.csv
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

## Step 3: Validate the Data

Run every row through the schema as a quality gate:

```bash
finetype validate data.csv data.schema.json
```

This produces three sidecar files (named after the input file):
- `data.csv.valid.csv` — rows that pass all validation rules (ready to load)
- `data.csv.invalid.csv` — rows that fail one or more rules (need attention)
- `data.csv.errors.jsonl` — machine-readable error records (row, column, rule, value)

**Note:** `finetype validate` exits with code 1 when any rows are invalid. This is a
data quality signal, not a command failure — check the report before deciding what to do.

**Read the validation report:**
- **Grade** (A–F) based on overall pass rate
- **Per-column breakdown** showing valid, invalid, and null counts
- A column at 91.7% valid means ~8% of values fail the pattern — investigate those

**Decision point — what to do with invalid rows:**

| Situation | Action |
|-----------|--------|
| Grade A/B (>80% valid) | Load `data.csv.valid.csv`, review invalids separately |
| Grade C/D (50–80%) | Investigate `data.csv.errors.jsonl` — the schema may be too strict or the data needs cleaning |
| Grade F (<50%) | Do not load. Check if the delimiter or encoding is wrong, or if the data needs preprocessing |

**Options:**
- `--summary-only` to skip writing sidecar files (just print the report)
- `-o json` for machine-readable summary

## Step 4: Load into DuckDB

Generate a `CREATE TABLE AS SELECT` with correct casts for every column:

```bash
finetype load -f data.csv.valid.csv
```

This prints runnable SQL to stdout. Pipe it to DuckDB:

```bash
finetype load -f data.csv.valid.csv > load.sql
duckdb mydb.db < load.sql
```

The generated SQL:
- Uses `read_csv('file.csv', all_varchar=true)` to read everything as strings first
- Applies the correct cast/transform per column (BIGINT, TIMESTAMP via strptime, DECIMAL with currency cleanup, etc.)
- Includes a trailing `SELECT * FROM table LIMIT 10` preview

**Options:**
- `--table-name my_table` to override the table name (default: filename)
- `--limit 0` to skip the preview SELECT
- `--no-normalize-names` to preserve original column names (DuckDB normalises by default)
- `--enum-threshold 50` to control when low-cardinality columns become ENUMs (0 = disable)

## Complete Pipeline Example

```bash
# 1. Profile — understand what you have
finetype profile -f contacts.csv

# 2. Schema — capture as a contract
finetype schema contacts.csv

# 3. Validate — quality gate (exit code 1 = invalid rows found, not a failure)
finetype validate contacts.csv contacts.schema.json

# 4. Load the clean rows
finetype load -f contacts.csv.valid.csv > load.sql
duckdb contacts.db < load.sql
```

## Quick Path (Skip Schema + Validate)

If you trust the data quality and just need typed SQL fast:

```bash
finetype load -f data.csv > load.sql
duckdb mydb.db < load.sql
```

`load` runs profile internally and generates SQL directly. Use this for exploratory work,
but prefer the full pipeline for production data where quality matters.

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
