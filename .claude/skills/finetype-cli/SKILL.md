---
name: finetype-cli
description: >
  FineType CLI reference — all commands, flags, and output formats.
  Use when you need to look up a specific command or flag.
user-invocable: false
---

# FineType CLI Reference

FineType v0.6.19 — Precision format detection for text data.

## Commands

### `finetype profile`

Profile a CSV file — detect column types using column-mode inference.

```bash
finetype profile -f <FILE> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file <FILE>` | *required* | Input CSV file |
| `-o, --output <FORMAT>` | `plain` | Output format: `plain`, `json`, `csv`, `markdown`, `arrow`, `json-schema` |
| `--sample-size <N>` | `100` | Max values to sample per column |
| `--delimiter <CHAR>` | auto-detect | CSV delimiter character |
| `--no-header-hint` | — | Disable column name header hints |
| `--model-type <TYPE>` | `multi-branch` | Model type: `multi-branch`, `char-cnn`, `tiered`, `transformer` |
| `--enum-threshold <N>` | `32` | Cardinality threshold for ENUM columns (0 = disable). Default lowered from 50 in v0.6.20 prep work to reduce enum-overfit attribution in profile→validate round-trip |
| `--stats` | — | Attach observed-data constraints (minLength, maxLength, minimum, maximum, enum, x-finetype-null-rate, x-finetype-cardinality) to JSON Schema output. Requires `-o json-schema` |
| `-v, --verbose` | — | Show pipeline tracing (Sense, mask, hint, feature decisions) |

**Output columns (plain):** COLUMN, TYPE (semantic label), BROAD (DuckDB type), CONF (confidence %)

**`-o json-schema`** — table-level JSON Schema for the file. Replaces the table-mode of the retired `finetype schema <file>` invocation (v0.6.19, MADR 0070).

---

### JSON Schema export — type-mode and table-mode

**v0.6.19 surface change:** the standalone `schema` verb was retired
(MADR 0070). JSON Schema export now splits across the two natural homes:

- **Type-mode** lives on `taxonomy`: `finetype taxonomy KEY -o json-schema`
- **Table-mode** lives on `profile`: `finetype profile -f FILE -o json-schema`

```bash
# Per-type schema (always emits a JSON array, even for single matches)
finetype taxonomy identity.person.email -o json-schema

# Glob — every identity.person.* type
finetype taxonomy "identity.person.*" -o json-schema

# Table-level schema for a CSV (profile pipeline; supports --stats)
finetype profile -f data.csv -o json-schema > schema.json
finetype profile -f data.csv -o json-schema --stats > schema.json
```

Both surfaces emit only `x-finetype-label` + `x-finetype-pii` on each
schema property (verbosity contract from PR #51 / card 0006). The
older `--pretty` flag is gone — `taxonomy -o json-schema` and
`profile -o json-schema` both pretty-print unconditionally.

The MCP `schema` tool's type-key branch is retained for v0.6.19; the
v0.6.20 audit will mirror the CLI fold (MADR 0070).

---

### `finetype validate`

Validate CSV (or Parquet) data against a JSON Schema. Default mode is
**check-only** — exit code communicates pass/fail; no files written.
Pass `--db <out.db> --table <name>` to materialise valid rows into a
typed DuckDB table alongside a `finetype_reject_errors` sidecar in the
same database — single pass, single CTAS.

```bash
finetype validate <FILE> <SCHEMA> [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--db <DB>` | — | Output DuckDB database file (created if absent). Optional — when omitted, validation runs in check-only mode. When supplied, `--table` is also required |
| `--table <TABLE>` | — | Table name in the output database for valid rows. Required when `--db` is supplied |
| `--append` | — | Append to an existing database. Required when `--db` already contains the named table or a prior `finetype_reject_errors` sidecar |
| `--lenient` | — | Force exit code 0 regardless of reject count (does not affect error exit code 2) |
| `-o, --output <FORMAT>` | `plain` | Summary-report format: `plain`, `json`, `csv`, `markdown`, `arrow`, `json-schema` |

**Materialisation behaviour** (when `--db --table` are passed):
- Valid rows → typed DuckDB table at `<DB>.<TABLE>`, per-column TRY-wrapped projection.
- Invalid rows → `finetype_reject_errors` table in the same `.db`, with `column_name`, `error_type`, `constraint_failed`, `expected_type`, `error_message` columns.
- Reject ontology: `error_type='SEMANTIC_TYPE'` (validator failure) and `error_type='TRANSFORM_FAILED'` with `constraint_failed='transform'` (validator passed, typed cast failed).

**Exit codes:** `0` no rejects · `1` rejects present (engine or transform) · `2` error (malformed schema, file unreadable, missing `duckdb` binary, etc.).

**Requirements:** materialisation requires `duckdb` on `PATH`. Check-only mode does not.

---

### `finetype load` *(removed in v0.6.19)*

Removed. The typed-output path now lives on `finetype validate
--db <out.db> --table <name>` — see the validate section above for the
typed CTAS, TRY-wrapped projection, and `finetype_reject_errors` sidecar
behaviour.

Migration:

```bash
# Before (v0.6.18 and earlier):
finetype load -f data.csv | duckdb mydb.db

# After (v0.6.19+):
finetype profile -f data.csv -o json-schema > schema.json
finetype validate data.csv schema.json --db mydb.db --table data
```

`finetype load …` now errors via clap's unknown-subcommand handler with
exit code 2. There is no shim or warning. See MADR 0071.

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
| `-o, --output <FORMAT>` | `plain` | Output format: `plain`, `json`, `csv`, `markdown`, `arrow`, `json-schema` |
| `--confidence` | — | Include confidence score |
| `-v, --value` | — | Include input value in output |
| `--mode <MODE>` | `row` | `row` (per-value) or `column` (distribution-based) |
| `--header <NAME>` | — | Column name for header hint (with `--mode column`) |
| `--sample-size <N>` | `100` | Sample size for column mode |
| `--batch` | — | Read JSONL from stdin (requires `--mode column`) |
| `--model-type <TYPE>` | `multi-branch` | Model type: `multi-branch`, `char-cnn`, `tiered`, `transformer` |
| `--bench` | — | Print throughput statistics to stderr |

**Row mode:** classifies each value independently.
**Column mode:** treats all inputs as one column, uses value distribution for disambiguation.

---

### `finetype taxonomy`

Show taxonomy information — browse available types, optionally filtered to a single type or glob.

```bash
finetype taxonomy [TYPE_KEY] [OPTIONS]
```

**Positional argument:**
- `[TYPE_KEY]` — type key (e.g. `identity.person.email`) or glob pattern (e.g. `identity.person.*`). When supplied, the `--domain` / `--category` / `--priority` filters are ignored.

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file <DIR>` | `labels` | Taxonomy file or directory |
| `-d, --domain <DOMAIN>` | — | Filter by domain |
| `-c, --category <CATEGORY>` | — | Filter by category |
| `--priority <N>` | — | Minimum release priority |
| `-o, --output <FORMAT>` | `plain` | Output format: `plain`, `json`, `csv`, `markdown`, `arrow`, `json-schema` |
| `--full` | — | Export all fields (description, validation, samples) |

**Domains:** container, datetime, finance, geography, identity, representation, technology

---

### `finetype generate` *(hidden — functional but absent from `finetype --help`)*

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

### `finetype check` *(hidden — functional but absent from `finetype --help`)*

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

No options. Runs as a stdio MCP server exposing the FineType CLI surface as
MCP tools (e.g. `infer`, `profile`, `taxonomy`, `validate`). The MCP tool
list mirrors the CLI surface; consult the server's tool-discovery response
for the authoritative inventory at runtime.

## Output Formats

All commands that accept `-o` support these formats:

| Format | Use |
|--------|-----|
| `plain` | Human-readable table (default) |
| `json` | Machine-readable, pipe to `jq` |
| `csv` | Comma-separated, pipe to other tools |
| `markdown` | Markdown table for documentation |
| `arrow` | Apache Arrow IPC for analytics tools |
| `json-schema` | JSON Schema export — table-mode on `profile`, type-mode on `taxonomy` (replaces the retired `schema` verb, MADR 0070) |

## Type Label Format

All FineType types use a three-part label: `domain.category.type`

- **domain** — broad area (container, datetime, finance, geography, identity, representation, technology)
- **category** — group within domain (person, timestamp, currency, internet, etc.)
- **type** — specific format (email, iso_8601, amount, ip_v4, etc.)

Example: `identity.person.email`, `datetime.timestamp.iso_8601`, `finance.currency.amount`

## Common Patterns

```bash
# Profile with JSON output for scripting
finetype profile -f data.csv -o json | jq '.columns[] | {name, type, confidence}'

# Schema for a specific type (not a file)
finetype taxonomy identity.person.email -o json-schema

# Schema-driven load with typed columns + reject sidecar (single pass)
finetype profile -f data.csv -o json-schema > data.schema.json
finetype validate data.csv data.schema.json --db mydb.db --table data

# Quality gate first (check-only), then materialise on PASS
finetype validate data.csv data.schema.json
finetype validate data.csv data.schema.json --db mydb.db --table data

# Classify values in column mode (better accuracy for ambiguous data)
finetype infer -f values.txt --mode column --header "amount"

# Explore types in a domain
finetype taxonomy -d identity --full -o json
```
