# Changelog

All notable changes to FineType will be documented in this file.

## [Unreleased]

## [0.1.5] - 2026-02-16

### Breaking Changes

- **Boolean taxonomy restructured** (NNFT-075): `technology.development.boolean` replaced by three format-specific subtypes:
  - `representation.boolean.binary` — 0/1 values
  - `representation.boolean.initials` — T/F, Y/N (single character, any case)
  - `representation.boolean.terms` — true/false, yes/no, on/off, enabled/disabled, active/inactive (any case)
  - All three map to DuckDB `BOOLEAN` type with normalization support
  - Legacy `technology.development.boolean` label is no longer emitted by the model

### Added

- **3 boolean subtypes** with dedicated generators producing case variants (NNFT-075)
- **Small-integer ordinal disambiguation** rule for columns like Pclass, ratings (NNFT-076)
- **30+ column header hints** for domain-specific columns: class/rank/tier, count/qty, survived/alive, ticket/cabin, fare/fee, embarked/terminal (NNFT-076)
- **Centralized `BOOLEAN_LABELS` constant** prevents label mismatch bugs across disambiguation rules (NNFT-076)
- **Early-development disclaimer** in README (NNFT-077)
- **Pre-commit hook** for automated fmt/clippy/test checks before commits
- 11 new tests for column disambiguation, header hints, boolean override behaviour

### Fixed

- **Boolean label mismatch** — `disambiguate_boolean_override()` was checking non-existent labels instead of actual model output (NNFT-076)
- Clippy warnings: `useless_format` in build.rs, `manual_range_contains` in generator.rs, `collapsible_str_replace` in column.rs

### Changed

- **CharCNN v6 model** trained on 169 types (up from 168), 89.15% accuracy
- Default model: `models/default` → `char-cnn-v6` (was char-cnn-v5)
- Taxonomy: 168 → 169 types (net +1: removed 1 boolean, added 3 boolean subtypes)
- Test suite: 213 tests (73 core + 109 model + 31 duckdb), up from 182
- DuckDB normalization: all three boolean subtypes routed to `normalize_boolean()`
- JSON boolean literals now annotated as `representation.boolean.terms` (was `technology.development.boolean`)

## [0.1.3] - 2026-02-15

### Added

- **7 financial identifier types** (NNFT-052): ISIN, CUSIP, SEDOL, SWIFT/BIC, LEI, ISO 4217 currency code, currency symbol
  - Check digit validation: Luhn (ISIN), weighted sum (CUSIP, SEDOL), ISO 7064 Mod 97-10 (LEI)
  - All types include DuckDB transformation contracts and decompose expressions
- **char-cnn-v4 model** trained on 159 types (up from 151) with v4 training data (129K samples)
  - Overall accuracy: 91.62%, Top-3: 99.21%
  - New type accuracy: LEI 96.6% F1, currency_code 94.3% F1, SEDOL 89.9% F1, CUSIP 84.6% F1
- 8 new unit tests for finance identifier generators with known-value verification

### Changed

- Default model updated: `models/default` → `char-cnn-v4` (was char-cnn-v2)
- Taxonomy expanded: 151 → 159 types
- Test suite: 73 unit tests (was 65)

### Known Issues

- `currency_symbol` type has low recall (2.5%) — single Unicode characters ($ € £) are confused with `emoji` by the character-level model. Post-processing rule planned.
- `isin` recall is 49.5% — 12-char ISINs starting with 2-letter country code confused with SWIFT/BIC codes

## [0.1.2] - 2026-02-13

### Added

- **Column-mode inference** with distribution-based disambiguation for ambiguous types (NNFT-012, NNFT-026)
- **Year disambiguation rule** — detects columns of 4-digit integers predominantly in 1900-2100 range (NNFT-026, NNFT-029)
- **Post-processing rules** — 6 deterministic format-based corrections applied after model inference (NNFT-033, NNFT-034, NNFT-035, NNFT-036):
  - RFC 3339 vs ISO 8601 offset (T vs space separator)
  - Cryptographic hash vs hex token (standard hash lengths: 32/40/64/128)
  - Emoji vs gender symbol (character identity check)
  - ISSN vs postal code (XXXX-XXX[0-9X] pattern)
  - Longitude vs latitude (out-of-range check for |value| > 90)
  - Email rescue (@ sign check for hostname/username/slug predictions)
- **`finetype profile`** command — detect column types in CSV files using column-mode inference (NNFT-027)
- **`finetype eval-gittables`** command — benchmark column-mode vs row-mode on GitTables real-world dataset (NNFT-028)
- **`finetype validate`** command — data quality validation against taxonomy schemas with quarantine/null/fill strategies
- **`models/default`** symlink — CLI now works with default `--model models/default` path out of the box
- **DuckDB extension functions**: `finetype_detail()`, `finetype_cast()`, `finetype_unpack()`, `finetype_version()` (NNFT-016, NNFT-017)
- Real-world evaluation against GitTables benchmark: 85-100% accuracy on format-detectable types (2,363 columns, 883 tables)

### Fixed

- Postal code rule no longer false-positives on year columns (NNFT-029)
- Year detection threshold relaxed from 100% to 80% to handle outliers (NNFT-032)
- Fixed accuracy number in documentation (91.97%, matching eval_results.json) (NNFT-031)
- Regenerated training/test data with corrected RFC 3339 format (space separator, not T) (NNFT-033)

### Improved

- Macro F1 improved from 87.9% to 90.8% via post-processing rules (+2.9 points without retraining)
- ISSN precision: 76% → 100%, recall: 73% → 97% (NNFT-035)
- Hash recall: 94.3% → 100% (NNFT-034)
- Emoji and gender symbol both reach 100% precision and recall (NNFT-034)
- Year generator range widened from 1990-2029 to 1800-2100 (NNFT-032)

### Changed

- README.md comprehensively updated with all 9 CLI commands, 5 DuckDB functions, column-mode docs (NNFT-030)
- DEVELOPMENT.md deprecated in favour of README + backlog tasks (NNFT-030)
- Column-mode disambiguation rules: date slash, coordinate, numeric types (port, increment, postal code, street number, year)
- Test suite expanded: 155 tests (65 core + 62 model + 28 CLI)

## [0.1.0] - 2026-02-11

### Initial Release

FineType is a semantic type classification engine for text data. Given any string value, it classifies the semantic type from a taxonomy of **151 types** across **6 domains**.

### Features

- **151 semantic types** across 6 domains: datetime (46), technology (34), identity (25), representation (19), geography (16), container (11)
- **Locale-aware taxonomy** with 16+ locales for dates, addresses, phone numbers
- **Flat CharCNN model** (char-cnn-v2): 91.97% test accuracy on 151 classes
- **Tiered hierarchical model**: 38 specialized models (Tier 0 broad type, Tier 1 category, Tier 2 specific type), 90.00% test accuracy
- **CLI commands**: `infer`, `generate`, `train`, `eval`, `check`, `taxonomy`
- **DuckDB extension** with embedded model weights — `finetype()` scalar function
- **Pure Rust** with Candle ML framework (no Python dependency)
- **Synthetic data generation** with priority-weighted sampling (500 samples/type default)
- **Taxonomy validation** via `finetype check` (validates YAML definitions, generators, regex patterns)
- **GitHub Actions CI/CD**: fmt, clippy, test, taxonomy check gates; cross-compile release workflow

### Taxonomy

Each type definition includes:
- Validation schema (regex + optional function)
- SQL transform/cast expression
- DuckDB target type
- Tier assignment for hierarchical models
- Locale assignments where applicable
- Example values and descriptions

### Model Architecture

- **CharCNN**: Character-level CNN with vocab=97, embed_dim=32, num_filters=64, kernel_sizes=[2,3,4,5], hidden_dim=128
- **Flat model**: Single 151-class classifier, 331KB safetensors weights
- **Tiered model**: Tier 0 (15 broad types, 98.02%) -> Tier 1 (5 trained + 10 direct-resolved) -> Tier 2 (32 models, 18 at 100%)

### Performance

- Model load: 66ms cold, 25-30ms warm
- Single inference: p50=26ms, p95=41ms (includes CLI startup)
- Batch throughput: 600-750 values/sec on CPU
- Memory: 8.5MB peak RSS
