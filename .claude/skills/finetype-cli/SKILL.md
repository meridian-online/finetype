---
name: finetype-cli
description: >
  FineType CLI reference — all commands, flags, and output formats.
  Use when you need to look up a specific command or flag.
user-invocable: false
---

# FineType CLI Reference

FineType v0.6.12 — Precision format detection for text data.

## Commands

### `finetype profile`

Profile a CSV file — detect column types using column-mode inference.

```bash
finetype profile -f <FILE> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file <FILE>` | *required* | Input CSV file |
| `-o, --output <FORMAT>` | `plain` | Output format: `plain`, `json`, `csv`, `markdown`, `arrow` |
| `-m, --model <DIR>` | `models/default` | Model directory |
| `--sample-size <N>` | `100` | Max values to sample per column |
| `--delimiter <CHAR>` | auto-detect | CSV delimiter character |
| `--no-header-hint` | — | Disable column name header hints |
| `--model-type <TYPE>` | `char-cnn` | Model type: `char-cnn`, `tiered`, `transformer` |
| `--sharp-only` | — | Disable Sense classifier (Sharpen-only with header hints) |
| `--enum-threshold <N>` | `50` | Cardinality threshold for ENUM columns (0 = disable) |
| `-v, --verbose` | — | Show pipeline tracing (Sense, mask, hint, feature decisions) |

**Output columns:** COLUMN, TYPE (semantic label), BROAD (DuckDB type), CONF (confidence %)

---

### `finetype schema`

Export JSON Schema for a type key or CSV file.

```bash
# For a single type
finetype schema <TYPE_KEY> [OPTIONS]

# For a CSV file (table mode)
finetype schema <FILE.csv> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file <DIR>` | `labels` | Taxonomy file or directory |
| `--pretty` | — | Pretty-print JSON output |
| `--stats` | — | Include observed data statistics (table mode only) |
| `--stdout` | — | Print to stdout instead of writing sidecar file (table mode only) |
| `-m, --model <DIR>` | `models/default` | Model directory (table mode only) |
| `--enum-threshold <N>` | `50` | Cardinality threshold for ENUM columns (table mode only) |

**Type key mode:** `finetype schema identity.person.email` — returns the JSON Schema for that type.

**Glob mode:** `finetype schema "identity.person.*"` — returns schemas for all matching types.

**Table mode:** `finetype schema data.csv` — profiles the file and writes `data.schema.json` with per-column validation rules, `x-finetype-label`, `x-finetype-broad-type`, `x-finetype-transform`, and `x-finetype-confidence`.

---

### `finetype validate`

Validate CSV data against a JSON Schema (quality gate).

```bash
finetype validate <FILE> <SCHEMA> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-o, --output <FORMAT>` | `plain` | Output format: `plain`, `json` |
| `--summary-only` | — | Print summary only — do not write sidecar files |

**Sidecar outputs** (appended to the full filename including extension):
- `<file>.csv.valid.csv` — rows passing all rules
- `<file>.csv.invalid.csv` — rows failing one or more rules
- `<file>.csv.errors.jsonl` — machine-readable error records

**Exit code:** 1 if any rows are invalid, 0 if all pass.

---

### `finetype load`

Generate runnable DuckDB `CREATE TABLE AS SELECT` from file profiling.

```bash
finetype load -f <FILE> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file <FILE>` | *required* | Input CSV file |
| `--table-name <NAME>` | from filename | Override table name |
| `-m, --model <DIR>` | `models/default` | Model directory |
| `--sample-size <N>` | `100` | Max values to sample per column |
| `--delimiter <CHAR>` | auto-detect | CSV delimiter character |
| `--no-header-hint` | — | Disable column name header hints |
| `--model-type <TYPE>` | `char-cnn` | Model type |
| `--sharp-only` | — | Disable Sense classifier |
| `--limit <N>` | `10` | Preview rows in trailing SELECT (0 = none) |
| `--no-normalize-names` | — | Preserve original column names |
| `--enum-threshold <N>` | `50` | Cardinality threshold for ENUM columns (0 = disable) |
| `-v, --verbose` | — | Enable pipeline tracing |

**Output:** SQL to stdout. Pipe to DuckDB: `finetype load -f data.csv | duckdb mydb.db`

---

### `finetype infer`

Classify text input — single values or files of values.

```bash
finetype infer [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-i, --input <TEXT>` | — | Single text input |
| `-f, --file <FILE>` | — | File of inputs (one per line) |
| `-o, --output <FORMAT>` | `plain` | Output format: `plain`, `json`, `csv`, `markdown`, `arrow` |
| `--confidence` | — | Include confidence score |
| `-v, --value` | — | Include input value in output |
| `--mode <MODE>` | `row` | `row` (per-value) or `column` (distribution-based) |
| `--header <NAME>` | — | Column name for header hint (with `--mode column`) |
| `--sample-size <N>` | `100` | Sample size for column mode |
| `--batch` | — | Read JSONL from stdin (requires `--mode column`) |
| `--model-type <TYPE>` | `char-cnn` | Model type |
| `--sharp-only` | — | Disable Sense classifier |
| `--bench` | — | Print throughput statistics to stderr |

**Row mode:** classifies each value independently.
**Column mode:** treats all inputs as one column, uses value distribution for disambiguation.

---

### `finetype taxonomy`

Show taxonomy information — browse available types.

```bash
finetype taxonomy [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file <DIR>` | `labels` | Taxonomy file or directory |
| `-d, --domain <DOMAIN>` | — | Filter by domain |
| `-c, --category <CATEGORY>` | — | Filter by category |
| `--priority <N>` | — | Minimum release priority |
| `-o, --output <FORMAT>` | `plain` | Output format: `plain`, `json`, `csv`, `markdown`, `arrow` |
| `--full` | — | Export all fields (description, validation, samples) |

**Domains:** datetime, finance, geography, identity, measurement, representation, technology

---

### `finetype generate`

Generate synthetic training data.

```bash
finetype generate [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-s, --samples <N>` | `100` | Samples per label |
| `-p, --priority <N>` | `3` | Minimum release priority |
| `-o, --output <FILE>` | `training.ndjson` | Output file |
| `-t, --taxonomy <DIR>` | `labels` | Taxonomy file or directory |
| `--seed <N>` | `42` | Random seed |
| `--localized` | — | Generate 4-level labels with locale suffixes |

---

### `finetype check`

Validate generator ↔ taxonomy alignment.

```bash
finetype check [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-t, --taxonomy <DIR>` | `labels` | Taxonomy file or directory |
| `-s, --samples <N>` | `50` | Samples per definition |
| `--seed <N>` | `42` | Random seed |
| `-p, --priority <N>` | — | Minimum release priority (0 = all) |
| `-v, --verbose` | — | Show verbose failure details |
| `-o, --output <FORMAT>` | `plain` | Output format: `plain`, `json` |

---

### `finetype mcp`

Start MCP server for AI agent integration (stdio transport).

```bash
finetype mcp
```

No options. Runs as a stdio MCP server exposing: `infer`, `profile`, `schema`, `taxonomy`, `validate`, `ddl`, `generate`.

## Output Formats

All commands that accept `-o` support these formats:

| Format | Use |
|--------|-----|
| `plain` | Human-readable table (default) |
| `json` | Machine-readable, pipe to `jq` |
| `csv` | Comma-separated, pipe to other tools |
| `markdown` | Markdown table for documentation |
| `arrow` | Apache Arrow IPC for analytics tools |

## Type Label Format

All FineType types use a three-part label: `domain.category.type`

- **domain** — broad area (identity, datetime, finance, geography, measurement, representation, technology)
- **category** — group within domain (person, timestamp, currency, internet, etc.)
- **type** — specific format (email, iso_8601, amount, ip_v4, etc.)

Example: `identity.person.email`, `datetime.timestamp.iso_8601`, `finance.currency.amount`

## Common Patterns

```bash
# Profile with JSON output for scripting
finetype profile -f data.csv -o json | jq '.columns[] | {name, type, confidence}'

# Schema for a specific type (not a file)
finetype schema identity.person.email --pretty

# Quick load without quality gate
finetype load -f data.csv | duckdb mydb.db

# Full pipeline with quality gate
finetype profile -f data.csv
finetype schema data.csv
finetype validate data.csv data.schema.json
finetype load -f data.csv.valid.csv > load.sql
duckdb mydb.db < load.sql

# Classify values in column mode (better accuracy for ambiguous data)
finetype infer -f values.txt --mode column --header "amount"

# Explore types in a domain
finetype taxonomy -d identity --full -o json
```
