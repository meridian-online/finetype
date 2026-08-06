# FineType

[![CI](https://github.com/meridian-online/finetype/actions/workflows/ci.yml/badge.svg)](https://github.com/meridian-online/finetype/actions/workflows/ci.yml)

> **Early Release** — FineType is under active development. Expect breaking changes to taxonomy labels, CLI arguments, library APIs, and model formats between releases. Pin to a specific version if stability matters for your use case.

Precision format detection for text data. FineType classifies strings into a rich taxonomy of 251 semantic types — each type is a **transformation contract** that guarantees a DuckDB cast expression will succeed.

```
# Point it at a whole column — the accurate path. A column types more
# accurately than a lone value: FineType samples the column and runs its
# guard → veto → recovery stack over the distribution, so it has more
# context than a single isolated value.
$ printf '541511\n541512\n236220\n' | finetype infer
representation.numeric.integer_number

# Single values work too — conclusive for value-determinable types:
$ finetype infer -i "192.168.1.1"
technology.internet.ip_v4

$ finetype infer -i "2024-01-15T10:30:00Z"
datetime.timestamp.iso_8601

$ finetype infer -i "hello@example.com"
identity.person.email
```

## Features

- **251 semantic types** across 7 domains — dates, times, IPs, emails, UUIDs, financial identifiers, currencies, geospatial formats, medical codes, and more
- **Transformation contracts** — each type maps to a DuckDB SQL expression that guarantees successful parsing. 99.9% actionability across 120 tested types.
- **Locale-aware** — validates 65 locales for postal codes, 46 for phone numbers, 27 for month/day names
- **MCP server** — `finetype mcp` exposes type inference to AI agents via [Model Context Protocol](https://modelcontextprotocol.io/)
- **DuckDB extension** — 12 scalar functions, 1 aggregate and 2 table macros, a `profile → schema → validate` surface in SQL: `ft_profile()` types a column or every column of a table, `ft_validate()` checks a table against a JSON Schema, plus `ft_infer()` / `ft_detail()` / `ft_cast()` / `ft_unpack()` scalars. The full table, gated against the loaded extension's catalog, is in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#duckdb-extension)
- **Schema-driven validation** — `finetype validate data.csv schema.json --db out.db --table orders` materialises typed DuckDB tables (per-column transforms applied) plus a `finetype_reject_errors` sidecar in a single pass
- **Pure Rust** — no Python runtime or dependencies

## Installation

> **Runtime dependency:** the [`duckdb`](https://duckdb.org/docs/installation) CLI must be on `PATH`. `finetype profile` and `finetype validate` shell out to it for all CSV/Parquet ingestion (`brew install duckdb`, or your platform package manager). This is a shell-out, not a link — the release binary is unchanged across platforms.

### Homebrew (macOS / Linux)

```bash
brew install meridian-online/tap/finetype
```

### Cargo

```bash
cargo install finetype-cli
```

### From Source

```bash
git clone https://github.com/meridian-online/finetype
cd finetype
cargo build --release
./target/release/finetype --version
```

## Usage

### CLI

```bash
# Classify a column of values — the accurate path (distribution-based).
# Pipe one value per line, or pass a file with -f. FineType samples the
# column and types from the whole distribution, so the guard/veto stack
# has more context than it would from a single value. Column is the
# default mode; add --confidence to see the sample count.
printf '541511\n541512\n236220\n' | finetype infer --confidence
#   representation.numeric.integer_number
#     confidence: 0.7933 (3 samples)
finetype infer -f column_values.txt --mode column

# Classify a single value (conclusive for value-determinable types)
finetype infer -i "bc89:60a9:23b8:c1e9:3924:56de:3eb1:3b90"

# Profile a CSV file — detect all column types
finetype profile -f data.csv

# Start MCP server for AI agent integration
finetype mcp

# Show taxonomy (filter by domain, category)
finetype taxonomy --domain datetime

# Export JSON Schema for a type (supports glob patterns)
finetype taxonomy "datetime.date.*" -o json-schema

# Validate a CSV against a JSON Schema — writes a DuckDB .db file with
# the user's typed table (valid rows, per-column transforms applied via
# TRY-wrapped projection) + `finetype_reject_errors` sidecar (engine
# rejects as error_type='SEMANTIC_TYPE'; cells that passed validation but
# failed the typed cast as error_type='TRANSFORM_FAILED').
# Exit codes: 0 no rejects / 1 rejects / 2 error. Requires `duckdb` on PATH.
finetype profile -f data.csv -o json-schema > schema.json
finetype validate data.csv schema.json --db out.db --table orders
duckdb out.db -c "SELECT column_name, error_type, constraint_failed, expected_type, type_confidence FROM finetype_reject_errors;"
```

### DuckDB Extension

```sql
INSTALL finetype FROM community;
LOAD finetype;
```

> **Version note:** the community channel builds per DuckDB version, so what you get depends on
> which DuckDB you run — and all of it lags this repo. Measured 2026-07-30 on `osx_arm64`:
>
> | DuckDB | `SELECT ft_version()` |
> |---|---|
> | 1.5.5 | `finetype 0.6.36` |
> | 1.5.4 | `finetype 0.6.36` |
> | 1.5.3 | `finetype 0.6.23` |
> | 1.5.2 | no build published — `INSTALL` fails with HTTP 404 |
>
> Rows are the 1.5 line. The 1.4 maintenance line has the same gaps — 1.4.4 is served, 1.4.5 is
> not — so always check what you actually got with `SELECT ft_version();` rather than assuming.
> To run the latest, build from source with `make build-release` and load it unsigned — see
> [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md#duckdb-extension-build).

```sql
-- Load a locally-built extension (DuckDB started with -unsigned)
LOAD './target/release/finetype.duckdb_extension';

-- The demo table every example below queries: a date column, a JSON column and
-- four rows. Paste the whole block — none of the examples creates it.
CREATE TABLE my_table (value VARCHAR, json_col VARCHAR);
INSERT INTO my_table VALUES
  ('01/15/2024', '{"host":"192.168.1.1","seen":"2024-01-15"}'),
  ('02/20/2024', '{"host":"10.0.0.8","seen":"2024-02-20"}'),
  ('03/25/2024', '{"host":"172.16.0.3","seen":"2024-03-25"}'),
  ('04/30/2024', '{"host":"192.168.1.9","seen":"2024-04-30"}');

-- Profile a whole table — one row per column. This is the everyday form.
SELECT * FROM ft_profile('my_table');
-- ┌─────────────┬─────────────────────────┬────────────────────┬─────────────┐
-- │ column_name │          type           │     confidence     │ duckdb_type │
-- │   varchar   │         varchar         │       double       │   varchar   │
-- ├─────────────┼─────────────────────────┼────────────────────┼─────────────┤
-- │ json_col    │ container.object.json   │ 0.7428288459777832 │ JSON        │
-- │ value       │ datetime.date.mdy_slash │ 0.6156769394874573 │ DATE        │
-- └─────────────┴─────────────────────────┴────────────────────┴─────────────┘
-- Box captured verbatim from a real run, type row and all, over the four rows
-- created above. `confidence` is a raw DOUBLE — ft_profile does not round
-- it, so expect the full value rather than the tidy 3dp that ft_detail's JSON
-- prints.
--
-- Do not expect the confidence DIGITS to reproduce, and note the box GEOMETRY
-- moves with them: a value one character longer widens the column and shifts
-- every border. Different builds disagree, and there is float jitter in the
-- low-order digits between runs of a single build. The column names, the
-- duckdb_type values and the type labels are the stable part.

-- Profile ONE column. ft_profile is an aggregate, so this is the plain SQL
-- shape: DuckDB pools the column into one sample and hands it over once.
SELECT ft_profile(value) FROM my_table;
-- {'type': datetime.date.mdy_slash, 'confidence': 0.833, 'duckdb_type': DATE}
-- A second argument is a header hint, fed to the model's header branch:
-- ft_profile(value, 'value'). GROUP BY, FILTER and DISTINCT all work; an
-- aggregate-level ORDER BY inside the call does not — see docs/DEVELOPMENT.md.

-- Validate a table against a JSON Schema (inline literal, variable, or file path)
SELECT * FROM ft_validate('my_table', 'schema.json');

-- Probe a single literal. No column context, so this is deliberately weaker
-- than ft_profile — reach for it to check one value, not to type a column.
SELECT ft_infer('192.168.1.1');
-- → 'technology.internet.ip_v4'

-- Full detail as JSON. Note this reads the COLUMN, not the one value in front
-- of it: the DuckDB processing chunk is the pooling boundary, so every row of a
-- chunk returns the SAME answer. It takes a STRIDED sample of up to 100 values
-- (ColumnConfig.sample_size) — evenly spaced, not the first 100 — and `samples`
-- reports how many it used. ft_detail(list(...)) samples the same way, so the
-- two agree.
--
-- ft_profile samples differently again, because an aggregate sees the rows one
-- chunk at a time and cannot stride over a column it has not read yet: it keeps
-- a reservoir of up to 100 values (PROFILE_SAMPLE_CAP), so a value late in the
-- scan is as likely to reach the model as one at the front. The two can
-- therefore disagree on a column whose values are not homogeneous.
SELECT ft_detail(value) FROM my_table;   -- 4-row date column
-- → {"type": "datetime.date.mdy_slash", "confidence": 0.833, "duckdb_type": "DATE", "samples": 4, "disambiguation": "date_slash_disambiguation", "votes": {"datetime.date.mdy_slash": 0.833}}
-- Every row returns that identical object. Note `SELECT … LIMIT 1` reports
-- "samples": 1, because the limit shrinks the chunk — not because it read one value.

-- Normalize values for safe TRY_CAST (dates → ISO, booleans → true/false)
SELECT ft_cast(value) FROM my_table;

-- Recursively classify JSON fields
SELECT ft_unpack(json_col) FROM my_table;
```

> **The model is column-oriented**, so `ft_profile` over a column is the accurate
> path and `ft_infer` over one literal is "profile with a sample size of one".
> Full surface, including the two table macros and the `profile → schema → validate`
> flow, in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md#duckdb-extension-sql-surface-ft_-verbs).

> **Deprecated:** the un-prefixed scalars (`finetype()`, `finetype_detail()`,
> `finetype_cast()`, `finetype_unpack()`, `finetype_validate()`, `finetype_version()`)
> are still registered as aliases but are no longer the taught surface — they have
> been superseded by the `ft_` verbs since 0.6.23.
>
> **None of these is a plain rename — check each one.** `finetype(value)` pools the DuckDB
> chunk as its sample, exactly as `ft_detail` does in scalar form, so it is *column*-level;
> `ft_infer` types a single value at a sample size of one and will give you different, weaker
> answers. If you were typing a column with `finetype()`, the replacement is **`ft_profile`**,
> and `ft_infer` only if you really were probing one literal.
>
> Separately, `finetype_validate` is **not** `ft_validate` (a table macro); its counterpart is
> **`ft_validate_text`** — and that swap **changes the return type**. `finetype_validate` returns
> a `VARCHAR`, the bare string `'valid'` or an error message; `ft_validate_text` returns a
> `STRUCT("valid" BOOLEAN, "constraint" VARCHAR, message VARCHAR)`. Code doing
> `WHERE finetype_validate(...) = 'valid'` must become `WHERE ft_validate_text(...).valid`.
>
> `finetype_detail` / `_cast` / `_unpack` / `_version` are true aliases of their `ft_` twins and
> are the only safe find-and-replace in the set.

On first use, the extension downloads model weights from HuggingFace and caches them locally. Set `FINETYPE_MODEL_DIR` to use a local model path instead.

### MCP Server

FineType exposes type inference to AI agents via the [Model Context Protocol](https://modelcontextprotocol.io/). Configure your MCP client to launch `finetype mcp` as a stdio subprocess.

The MCP tool surface mirrors the CLI (one capability surface, enforced by a parity-guard test):

| Tool | Purpose |
|---|---|
| `infer` | Classify values (single or column mode with header) |
| `profile` | Profile all columns in a CSV/Parquet/JSON file (path or inline data); `format: "json-schema"` for a table-level schema |
| `taxonomy` | Search/filter the type taxonomy; with a `key`/glob + `format: "json-schema"`, export per-type JSON Schema |
| `validate` | Schema-driven CSV validation — valid/invalid counts + error details |
| `generate` | Generate synthetic sample data for a type |

**Resources:** `finetype://taxonomy`, `finetype://taxonomy/{domain}`, `finetype://taxonomy/{domain}.{category}.{type}`

### As a Library

The shipped model is the column-level multi-branch classifier paired with a
Model2Vec header encoder:

```rust
use finetype_model::{ColumnClassifier, ColumnConfig, Model2VecResources, MultiBranchClassifier};

let mb = MultiBranchClassifier::load("models/default")?;
let mut classifier = ColumnClassifier::with_multi_branch(mb, ColumnConfig::default());
classifier.set_model2vec(Model2VecResources::load("models/model2vec")?);

let result = classifier.classify_column(&["hello@example.com".to_string()])?;
println!("{} (confidence: {:.2})", result.label, result.confidence);
// → identity.person.email (confidence: 0.97)
```

## Taxonomy

FineType recognizes **251 types** across **7 domains**:

| Domain | Types | Examples |
|--------|-------|----------|
| `datetime` | 89 | ISO 8601, RFC 2822, Unix timestamps, CJK dates, Apache CLF, timezones, month/day names (27 locales) |
| `representation` | 32 | Integers, floats, booleans, numeric codes, hex colors, JSON, CAS numbers, SMILES, InChI |
| `technology` | 30 | IPv4/v6, MAC, URLs, UUIDs, ULIDs, DOIs, hashes, JWTs, AWS ARNs, Docker refs, CIDRs, git SHAs |
| `identity` | 34 | Names, emails, phone numbers (46 locales), credit cards, SSNs, VINs, medical codes (ICD-10, CPT, LOINC) |
| `finance` | 29 | IBAN, SWIFT/BIC, ISIN, CUSIP, SEDOL, LEI, FIGI, currency amounts (7 format variants), routing numbers |
| `geography` | 25 | Lat/lon, countries, cities, postal codes (65 locales), WKT, GeoJSON, H3, geohash, Plus Codes, MGRS |
| `container` | 12 | JSON objects, CSV rows, query strings, key-value pairs |

Each type is a **transformation contract** — if FineType predicts `datetime.date.mdy_slash`, that guarantees `strptime(value, '%m/%d/%Y')::DATE` will succeed.

Label format: `{domain}.{category}.{type}` (e.g., `technology.internet.ip_v4`). Locale-specific types append a locale suffix: `identity.person.phone_number.EN_AU`.

See [`labels/`](labels/) for the complete taxonomy definitions.

## Performance

FineType runs a two-stage pipeline: a **semantic** classifier (the multi-branch
model) predicts a broad type, then a **deterministic** refinement layer
(value-based rules and vetoes) sharpens it to a leaf type and enforces the
transformation contract.

| Metric | Value |
|--------|-------|
| Accuracy | 0.81 on the 931-column human-verified gold corpus |
| Actionability | 99.9% (232,321/232,541 values transformed across 120 types) |
| Model classes | 244 |
| Profile (model load + classify) | ~150 ms; ~180 MB peak RSS |

The dual-encoder model (potion-8M value branch + potion-4M header branch) is the
dominant footprint; value-determinable inputs (emails, IPs, ISO datetimes) take a
deterministic fast path that skips the model load entirely (~50 ms).

## Known Limitations

### DuckDB `strptime` Locale Limitation

DuckDB's `strptime` function only accepts English month and day names. Non-English dates like `6 janvier 2025` will fail with `strptime(col, '%d %B %Y')`.

**Affected types:** `datetime.date.long_full_month`, `datetime.date.abbreviated_month`, and related timestamp variants with non-English month/day names.

**Workaround:** FineType's locale detection correctly identifies non-English dates, but transformation must normalize to English first. See [Locale Support Guide](docs/LOCALE_GUIDE.md) for details.

## Development

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for training pipelines, DuckDB extension builds, and contributor setup. For architecture details, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

```bash
cargo build --release          # Build
cargo test --all               # Run tests
cargo run --release -- check   # Validate taxonomy alignment
make eval-report               # Run evaluation suite
```

## License

MIT — see [`LICENSE`](LICENSE)

## Contributing

Contributions welcome! Please open an issue or PR.

## Credits

Part of the [Meridian](https://meridian.online) project.

Built with [Candle](https://github.com/huggingface/candle) (Rust ML), [DuckDB](https://duckdb.org), [QSV](https://github.com/dathere/qsv) (CSV toolkit), [rmcp](https://github.com/modelcontextprotocol/rust-sdk) (MCP SDK), and [Serde](https://serde.rs).

Training TUI dashboard inspired by [Burn](https://github.com/tracel-ai/burn)'s training renderer ([`burn-train`](https://github.com/tracel-ai/burn/tree/main/crates/burn-train/src/renderer/tui)).
