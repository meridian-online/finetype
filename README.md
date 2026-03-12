# FineType

[![CI](https://github.com/meridian-online/finetype/actions/workflows/ci.yml/badge.svg)](https://github.com/meridian-online/finetype/actions/workflows/ci.yml)

Precision format detection for text data. FineType classifies strings into a rich taxonomy of 250 semantic types — each type is a **transformation contract** that guarantees a DuckDB cast expression will succeed.

```
$ finetype infer -i "192.168.1.1"
technology.internet.ip_v4

$ finetype infer -i "2024-01-15T10:30:00Z"
datetime.timestamp.iso_8601

$ finetype infer -i "hello@example.com"
identity.person.email
```

## Features

- **250 semantic types** across 7 domains — dates, times, IPs, emails, UUIDs, financial identifiers, currencies, geospatial formats, medical codes, and more
- **Transformation contracts** — each type maps to a DuckDB SQL expression that guarantees successful parsing. 99.9% actionability across 120 tested types.
- **Locale-aware** — validates 65+ locales for postal codes, 46+ for phone numbers, 32+ for month/day names
- **MCP server** — `finetype mcp` exposes type inference to AI agents via [Model Context Protocol](https://modelcontextprotocol.io/)
- **DuckDB extension** — `finetype()`, `finetype_detail()`, `finetype_cast()`, `finetype_unpack()` scalar functions
- **DuckDB load** — `finetype load -f data.csv | duckdb` generates runnable CREATE TABLE statements
- **Pure Rust** — no Python runtime or dependencies

## Installation

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
# Classify a single value
finetype infer -i "bc89:60a9:23b8:c1e9:3924:56de:3eb1:3b90"

# Profile a CSV file — detect all column types
finetype profile -f data.csv

# Generate a runnable DuckDB CREATE TABLE from file profiling
finetype load -f data.csv | duckdb

# Column-mode inference (distribution-based disambiguation)
finetype infer -f column_values.txt --mode column

# Start MCP server for AI agent integration
finetype mcp

# Show taxonomy (filter by domain, category)
finetype taxonomy --domain datetime

# Export JSON Schema for a type (supports glob patterns)
finetype schema "datetime.date.*" --pretty
```

### DuckDB Extension

```sql
-- Install and load
INSTALL finetype FROM community;
LOAD finetype;

-- Classify a single value
SELECT finetype('192.168.1.1');
-- → 'technology.internet.ip_v4'

-- Classify a column with detailed output (type, confidence, DuckDB broad type)
SELECT finetype_detail(value) FROM my_table;
-- → '{"type":"datetime.date.mdy_slash","confidence":0.98,"broad_type":"DATE"}'

-- Normalize values for safe TRY_CAST (dates → ISO, booleans → true/false)
SELECT finetype_cast(value) FROM my_table;

-- Recursively classify JSON fields
SELECT finetype_unpack(json_col) FROM my_table;
```

The extension embeds model weights at compile time — no external files needed.

### MCP Server

FineType exposes type inference to AI agents via the [Model Context Protocol](https://modelcontextprotocol.io/). Configure your MCP client to launch `finetype mcp` as a stdio subprocess.

| Tool | Purpose |
|---|---|
| `infer` | Classify values (single or column mode with header) |
| `profile` | Profile all columns in CSV file (path or inline data) |
| `ddl` | Generate CREATE TABLE DDL from file profiling |
| `taxonomy` | Search/filter type taxonomy by domain/category/query |
| `schema` | Export JSON Schema contract for type(s), supports globs |
| `generate` | Generate synthetic sample data for a type |

**Resources:** `finetype://taxonomy`, `finetype://taxonomy/{domain}`, `finetype://taxonomy/{domain}.{category}.{type}`

### As a Library

```rust
use finetype_model::Classifier;

let classifier = Classifier::load("models/default")?;
let result = classifier.classify("hello@example.com")?;

println!("{} (confidence: {:.2})", result.label, result.confidence);
// → identity.person.email (confidence: 0.97)
```

## Taxonomy

FineType recognizes **250 types** across **7 domains**:

| Domain | Types | Examples |
|--------|-------|----------|
| `datetime` | 84 | ISO 8601, RFC 2822, Unix timestamps, CJK dates, Apache CLF, timezones, month/day names (32+ locales) |
| `representation` | 36 | Integers, floats, booleans, numeric codes, hex colors, JSON, CAS numbers, SMILES, InChI |
| `technology` | 28 | IPv4/v6, MAC, URLs, UUIDs, ULIDs, DOIs, hashes, JWTs, AWS ARNs, Docker refs, CIDRs, git SHAs |
| `identity` | 34 | Names, emails, phone numbers (46+ locales), credit cards, SSNs, VINs, medical codes (ICD-10, CPT, LOINC) |
| `finance` | 31 | IBAN, SWIFT/BIC, ISIN, CUSIP, SEDOL, LEI, FIGI, currency amounts (7 format variants), routing numbers |
| `geography` | 25 | Lat/lon, countries, cities, postal codes (65+ locales), WKT, GeoJSON, H3, geohash, Plus Codes, MGRS |
| `container` | 12 | JSON objects, CSV rows, query strings, key-value pairs |

Each type is a **transformation contract** — if FineType predicts `datetime.date.mdy_slash`, that guarantees `strptime(value, '%m/%d/%Y')::DATE` will succeed.

Label format: `{domain}.{category}.{type}` (e.g., `technology.internet.ip_v4`). Locale-specific types append a locale suffix: `identity.person.phone_number.EN_AU`.

See [`labels/`](labels/) for the complete taxonomy definitions.

## Performance

| Model | Profile Eval | Actionability | Classes |
|-------|----------|---------|---------|
| **Sense→Sharpen** (default) | **97.7% label** (170/174) | **99.9%** | **250** |
| Tiered v2 (`--sharp-only`) | Legacy fallback | — | 164 |

**Profile eval:** 30 real-world datasets, 174 format-detectable columns. **Actionability:** 232,321/232,541 values transformed successfully across 120 types.

| Metric | Value |
|--------|-------|
| Model load time | 66 ms (cold), 25-30 ms (warm) |
| Single inference | p50=26 ms, p95=41 ms (includes CLI startup) |
| Batch throughput | 600-750 values/sec on CPU |
| Memory footprint | 8.5 MB peak RSS |

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

Built with [Candle](https://github.com/huggingface/candle) (Rust ML), [DuckDB](https://duckdb.org), [rmcp](https://github.com/modelcontextprotocol/rust-sdk) (MCP SDK), and [Serde](https://serde.rs).
