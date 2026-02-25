# Changelog

All notable changes to FineType will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.2] - 2026-02-25

### Accuracy

- **Locale-aware phone number validation** — per-locale validation patterns (14 locales) for phone_number integrated into attractor demotion Signal 1. Patterns derived from Google libphonenumber (Apache 2.0), embedded in taxonomy YAML. Phone_number added to TEXT_ATTRACTORS enabling demotion of false positives while locale-confirmed predictions are preserved (NNFT-121)

## [0.2.1] - 2026-02-25

### Accuracy

- **Locale-aware postal code validation** — per-locale validation patterns (14 locales) integrated into attractor demotion Signal 1. Locale-confirmed predictions skip demotion. Patterns sourced from Google libaddressinput (Apache 2.0), embedded in taxonomy YAML (NNFT-118)
- **Model2Vec threshold tuned** — lowered from 0.70 to 0.65, recovering 12 additional correct semantic matches (timezone, postal codes, status codes, price variants) with one accepted borderline FP (data→form_data at 0.687) (NNFT-122)
- **Targeted synonyms** — added header hint synonyms for IANA timezone, postal code, URL, HTTP status code, and MIME type to improve column name matching (NNFT-123)

### Changed

- **Max-sim matching for Model2Vec** — replaced mean-pooled single centroids with K=3 representative embeddings per type using Farthest Point Sampling (FPS). Eliminates centroid dilution from diverse synonyms. `type_embeddings.safetensors` uses interleaved layout `[n_types*K, embed_dim]`; K inferred at load time for backward compatibility with K=1 artifacts. `prepare_model2vec.py` adds `--max-k` and `--legacy` flags (NNFT-124)

## [0.2.0] - 2026-02-24

### Accuracy

- **Multi-signal attractor demotion** — Rule 14 demotes over-eager specific type predictions (postal_code, cvv, first_name, icao_code) to generic types using validation failure, confidence threshold, and cardinality signals. 17 predictions improved, 0 format-detectable regressions (NNFT-115)
- **Numeric range validation** — added `maximum: 99999` constraint to postal_code and street_number validation schemas, eliminating false positives on salary, ticket number, and byte count columns (NNFT-117)

### Changed

- **JSON Schema validation engine** — migrated from hand-rolled regex to `jsonschema` crate (v0.42.1, pure Rust, Draft 2020-12). `CompiledValidator` pre-compiles schemas once; taxonomy caches validators via `compile_validators()`. Hybrid strategy: string keywords delegated to jsonschema, numeric bounds handled manually for string→f64 parsing. Enables future `format`, `oneOf`, `if/then` keywords (NNFT-116)

## [0.1.9] - 2026-02-24

### Added

- **Model2Vec semantic header hints** — column name classification using Model2Vec static embeddings (potion-base-4M, 7.4MB float16) with cosine similarity against pre-computed type embeddings. Threshold 0.70 tuned for zero false positives on generics (NNFT-110)
- **Unified column-level disambiguation** — consolidated all column disambiguation rules into a single pipeline. Profile eval 55/74 → 68/74 format-detectable correct (+13, 0 regressions) (NNFT-109)

### Changed

- **DuckDB community extension v0.2.0** — updated with tiered model, 168 types, 19 new DuckDB type mappings (NNFT-092)
- finetype-core and finetype-model published to crates.io at v0.1.9 (NNFT-114)

## [0.1.8] - 2026-02-18

### Performance

- **30× tiered inference throughput** — group-then-batch processing in `classify_batch()` improves from ~17 to ~580 val/sec; flat model ~1,500 val/sec (NNFT-098)
- **Batched CLI inference** — all model types process in chunks of 128 (was per-value)
- **`--bench` flag** — prints throughput and per-tier timing breakdown to stderr (NNFT-098)
- **`TierTiming` struct** — public API for per-tier performance measurement

### Accuracy (72.6% → 92.9%)

- **`header_hint_generic` override** — header hints now override generic model predictions (integer, username, phone_number, iata_code, etc.) even when the hinted type isn't in the vote distribution. This single change lifted accuracy by +7.1pp (NNFT-102)
- **IPv4 disambiguation rule** — dotted-quad pattern detection with 0–255 octet validation (NNFT-100)
- **Day/month/boolean disambiguation** — value-level rules for day-of-week names, month names, and boolean sub-type normalization (NNFT-090)
- **Gender expansion** — +22 inclusive values (Non-binary, Other, Prefer not to say, etc.) (NNFT-099)
- **Expanded header hints** — alpha-2/3 country codes, occupation/job title, IP variants, UTC offset, CVV/SWIFT/ISSN/EAN/NPI, weight/height, OS, subcountry (NNFT-100, NNFT-102)
- **Expanded `is_generic`** — phone_number, iata_code, and increment added (NNFT-100, NNFT-102)
- **Eval scoring interchangeability** — boolean sub-types, time sub-types, geographic hierarchy, timestamp precision (NNFT-099, NNFT-100)

### Fixed

- **Column mode with tiered model** — `--mode column` now works with all model types via `Box<dyn ValueClassifier>`; was char-cnn only, broken since v0.1.7 default change (NNFT-101)
- **Windows build.rs symlink resolution** — `read_link` fallback now reads `models/default` as plain text file when symlink isn't available (git on Windows checks out symlinks as text files) (NNFT-094)

### Changed

- **`--model-type` help text** documents performance/accuracy tradeoff (~600 vs ~1,500 val/sec)
- **Windows release target** — `x86_64-pc-windows-msvc` added to release CI matrix (NNFT-094)
- `download-model.sh` gains `readlink`/`cat` fallback for Windows symlink compatibility
- Release workflow steps use explicit `shell: bash` for cross-platform builds

## [0.1.7] - 2026-02-18

### Added

- **Tiered model graph** as default inference engine — 34 specialized CharCNN models in a hierarchical T0→T1→T2 architecture (NNFT-084, NNFT-087)
- **`ValueClassifier` trait** — polymorphic dispatch enabling both flat `CharClassifier` and `TieredClassifier` through a single interface (NNFT-084)
- **SI number disambiguation** — improved handling of values with SI prefixes in tiered profile evaluation (NNFT-084)

### Changed

- Default model: `models/default` → `char-cnn-v5` tiered (was `char-cnn-v6` flat)
- Profile evaluation improved by +4.5 percentage points with tiered model
- Inference engine: single flat classifier replaced by tiered graph dispatch

## [0.1.6] - 2026-02-17

### Added

- **Automated profile-and-compare evaluation pipeline** — benchmark column detection across model versions (NNFT-080)
- **20 curated benchmark datasets** with 206 ground truth column annotations (NNFT-081)
- **Machine-readable type mapping** — schema.org/DBpedia → FineType crosswalk for external taxonomy alignment (NNFT-079)

### Fixed

- **Numeric type disambiguation** — fixed training label mapping bug causing incorrect type resolution (NNFT-083)

### Changed

- Expanded GitTables 1M evaluation with CharCNN v6 (NNFT-082)

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

## [0.1.4] - 2026-02-16

### Added

- **17 new taxonomy types** expanding coverage to 168 types:
  - Medical identifiers: DEA number, NDC, NPI (NNFT-053)
  - SI-prefix numbers: `representation.numeric.si_number` (NNFT-057)
  - Excel custom number format detection: `representation.file.excel_format` (NNFT-059)
  - Expanded phone number generator with NATIONAL/INTL/E164 formats (NNFT-055)
  - Expanded address generator with locale-specific format templates (NNFT-056)
  - Categorical, ordinal, and alphanumeric_id types (NNFT-063)
  - Name format diversity and designation audit (NNFT-066, NNFT-070)
- **Pattern-gated post-processing** using taxonomy validation patterns for deterministic corrections (NNFT-064)
- **Column-name header hints** as soft inference signal for ambiguous types (NNFT-067)
- **Cardinality disambiguation** for low-cardinality columns (NNFT-065)
- **Per-topic evaluation harnesses** for GitTables 1M (NNFT-041)
- **GitTables 1M formalized** as standard evaluation benchmark (NNFT-040)
- **Pre-commit hook** infrastructure with `.githooks/pre-commit` and Makefile setup target
- Embedded taxonomy in binary; developer-only CLI commands hidden from help

### Fixed

- **Port disambiguation false positive** on age/count columns (NNFT-062)
- Windows build.rs: normalized backslash paths in `include_bytes!()` macros
- Smoke test URL assertion for v5 taxonomy label changes

### Changed

- Taxonomy expanded: 159 → 168 types
- CharCNN v5 model trained on 168 types, 90.09% accuracy
- Default model: `models/default` → `char-cnn-v5` (was char-cnn-v4)
- Dynamic model download from HuggingFace in CI/release workflows

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

## [0.1.2] - 2026-02-14

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
- **DOI type** — `technology.code.doi` with regex validation and Crossref decompose expression

### Fixed

- Postal code rule no longer false-positives on year columns (NNFT-029)
- Year detection threshold relaxed from 100% to 80% to handle outliers (NNFT-032)
- Fixed accuracy number in documentation (91.97%, matching eval_results.json) (NNFT-031)
- Regenerated training/test data with corrected RFC 3339 format (space separator, not T) (NNFT-033)
- Profile command output formatting and edge cases

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
- Homebrew formula auto-updated on release via CI workflow

## [0.1.1] - 2026-02-13

### Added

- **Embedded model** in CLI binary — `finetype infer` works standalone without external model files (NNFT-020)
- **Published to crates.io** — finetype-core and finetype-model available as Rust library crates
- **Published to HuggingFace** — model weights hosted at noon-org/finetype-char-cnn
- **CI model download** — release and CI workflows fetch model from HuggingFace instead of bundling in git
- **CLI smoke tests** for release validation (NNFT-047)

### Changed

- Build system: model weights embedded via `include_bytes!()` in build.rs
- CI/release workflows updated to download model before build

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
