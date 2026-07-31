# finetype-core

Core library of [FineType](https://github.com/meridian-online/finetype) — a
type inference engine that detects and classifies semantic types in tabular
data (is this column an ISIN, a NAICS industry code, a latitude, an ISO
timestamp?).

This crate carries the pieces that don't need model weights:

- **The type taxonomy** — 251 semantic type definitions across seven domains
  (datetime, finance, geography, identity, representation, technology,
  container), each with a JSON-Schema validation fragment, DuckDB transform
  expressions, and format metadata. Load from YAML with [`Taxonomy`].
- **Validators** — pre-compiled per-type validation (`CompiledValidator`),
  plus substance checks the shape patterns can't provide: real check-digit
  algorithms (`checksum`: ISBN, ISIN, LEI, IBAN, FIGI, CUSIP, SEDOL, ABA,
  Luhn) and closed-set membership against published code lists
  (`membership`: ICAO/IATA airports, NAICS industry codes).
- **Deterministic detectors** — datetime format resolution, locale data, and
  the synthetic data generator used for training.

## Example

```rust
use finetype_core::taxonomy::Taxonomy;

let yaml = r#"
finance.currency.currency_code:
  title: "Currency Code (ISO 4217)"
  validation:
    type: string
    enum: [USD, EUR, GBP, JPY, AUD]
  tier: [VARCHAR, currency]
  release_priority: 4
  samples: ["USD"]
"#;

let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
taxonomy.compile_validators();

let validator = taxonomy.get_validator("finance.currency.currency_code").unwrap();
assert!(validator.is_valid("USD"));
assert!(!validator.is_valid("XYZ"));
```

Check-digit and membership substance checks are plain functions:

```rust
assert!(finetype_core::checksum::isin("US0378331005"));   // real check digit
assert!(!finetype_core::checksum::isin("US0378331009"));  // right shape, wrong digit
assert!(finetype_core::membership::naics_codes("541511")); // published NAICS code
assert!(!finetype_core::membership::icao_airports("AAPL")); // a ticker, not an airport
```

## Feature flags

- `embedded-taxonomy` — embeds the taxonomy YAMLs at compile time. This is
  **workspace-only** (it reads the repository's `labels/` directory) and is
  off in the published crate; load the taxonomy from YAML strings or files
  instead.

## The wider project

The trained classifier lives in
[`finetype-model`](https://crates.io/crates/finetype-model); the `finetype`
CLI (profile CSV/Parquet files, generate schemas, validate and materialise
typed output) ships via
[GitHub releases and Homebrew](https://github.com/meridian-online/finetype#installation).

License: MIT
