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
- **DuckDB extension** — 13 scalar functions and 2 table macros: `ft_profile()` over an assembled list or a whole table, `ft_validate()`, `ft_infer()`, `ft_detail()`, `ft_cast()`, `ft_unpack()`, `ft_validate_text()`, `ft_version()`, and the un-prefixed `finetype*` names as aliases. The full table, gated against the loaded extension's catalog, is in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#duckdb-extension)
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

> **Install:** the signed community build (`INSTALL finetype FROM community;`) is being republished and is currently unavailable on recent DuckDB versions. Until it lands, build the extension from this repo with `make build-release` and load it unsigned — see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md#duckdb-extension-build).

```sql
-- Load a locally-built extension (DuckDB started with -unsigned)
LOAD './target/release/finetype.duckdb_extension';

-- Classify a single value
SELECT finetype('192.168.1.1');
-- → 'technology.internet.ip_v4'

-- Classify a column with detailed output (type, confidence, DuckDB type)
SELECT finetype_detail(value) FROM my_table;
-- → '{"type":"datetime.date.mdy_slash","confidence":0.98,"duckdb_type":"DATE","samples":4,"votes":{"datetime.date.mdy_slash":0.98}}'

-- Normalize values for safe TRY_CAST (dates → ISO, booleans → true/false)
SELECT finetype_cast(value) FROM my_table;

-- Recursively classify JSON fields
SELECT finetype_unpack(json_col) FROM my_table;
```

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
