# Changelog

All notable changes to FineType will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.6.8] - 2026-03-08

### Improved

- **Profile accuracy: 96.2% label, 98.4% domain** (179/186 columns, up from 178/186). ~30 new header hints for epoch/unix timestamps, age, altitude, duration, attendance, categorical text (language, sport, species, exchange). Cross-domain hardcoded hint override with domain-aware thresholds (0.85 cross-domain, 0.5 same-domain). 7 substring matching bug fixes ("count" vs "country", "address" vs "mac_address", etc.). (NNFT-254)

### Added

- **Golden integration test suite** — 13 structured Rust integration tests covering `profile`, `load`, `taxonomy`, and `schema` commands. 4 real-world dataset tests (datetime_formats, ecommerce_orders, titanic, people_directory), 3 focused fixture tests (ambiguous headers, numeric edge cases, categoricals), 2 load DDL tests, 2 taxonomy tests, 2 schema tests. Gated with `#[ignore]` for fast dev workflow. (NNFT-258)

### Discovery

- **Feature-augmented retrain confirmed: keep rules** — NNFT-254 confirmed that feature_dim=0 + expanded header hints outperforms feature-augmented CharCNN (which regresses -1.6pp due to city attractor). Decided item #22: rules over feature-augmented model.

## [0.6.7] - 2026-03-08

### Added

- **Feature-augmented inference pipeline** — 32 deterministic features (parse tests, character statistics, structural patterns) extracted per value and used for post-vote disambiguation. Three rules: F1 leading-zero detection (postal_code/cpt → numeric_code), F2 slash-segment counting (hostname → docker_ref), F3 digit-ratio + dot pattern (decimal_number → hs_code). Features are computed in the Sense→Sharpen pipeline alongside CharCNN classification. (NNFT-247, NNFT-250)
- **CharCNN feature fusion architecture** — `feature_dim` config parameter enables parallel feature vector fusion at the classifier head (fc1 input = total_filters + feature_dim). Backward compatible: `feature_dim=0` (default) preserves existing model behaviour. Training pipeline supports `--use-features` flag. (NNFT-248, NNFT-249)

### Fixed

- **`finetype load` CAST for generic numeric types** — Types like `decimal_number` (DOUBLE) and `integer_number` (BIGINT) were output as bare VARCHAR because `is_generic` conflated classification uncertainty with cast safety. Now broad_type flows directly from taxonomy — all non-VARCHAR types get their CAST applied. (NNFT-252)

### Accuracy

- **Profile eval: 95.7% label, 97.3% domain** (178/186 columns). Feature disambiguation rules resolved cpt (100%), hs_code (100%), and docker_ref (100%) confusion pairs. (NNFT-251)
- **Actionability eval: 99.9%** — 232,321/232,541 values transformed successfully. (NNFT-251)

### Discovery

- **Feature-augmented retrain (NNFT-253)** — Training CharCNN with feature_dim=32 improves training accuracy +5pp (86.6% → 91.6%) but regresses profile eval -1.6pp due to city attractor from character statistic features. Recommendation: keep feature_dim=0 with post-vote rules. See `discovery/feature-retrain/FINDING.md`.

## [0.6.6] - 2026-03-08

### Added

- **`finetype load` command** — Generates runnable DuckDB `CREATE TABLE AS SELECT` statements from file profiling. Pipe directly into DuckDB: `finetype load -f data.csv | duckdb`. Features: taxonomy transform expressions for typed columns, column name normalization via SQL aliases (default on, `--no-normalize-names` to opt out), trailing `SELECT * LIMIT 10` preview (`--limit N` to control), `all_varchar=true` for FineType-controlled type casting. (NNFT-238)
- **`profile -o arrow`** — Arrow IPC JSON schema output format, moved from the retired `schema-for` command. (NNFT-239)

### Removed

- **`schema-for` command** — Retired entirely. Its three output modes are now covered by `load` (runnable CTAS), `profile -o json` (superset with confidence/locale/quality), and `profile -o arrow`. No deprecation period — command was young with no known external consumers. (NNFT-239)

## [0.6.5] - 2026-03-07

### Fixed

- **Missing taxonomy definitions** — 25 types (10 geography, 15 identity) from NNFT-244 were not embedded in the v0.6.4 binary due to uncommitted YAML files. Taxonomy now correctly includes all 250 types.

## [0.6.4] - 2026-03-07

### Added

- **MCP server** — `finetype mcp` subcommand exposing type inference to AI agents via Model Context Protocol. 6 tools (infer, profile, ddl, taxonomy, schema, generate) + taxonomy resources. Built on rmcp v1.1.0, stdio transport, JSON + markdown dual output. (NNFT-241)
- **Taxonomy expansion to 250 types** — 43 new type definitions across all domains: geography +10 (wkt, geojson, h3, geohash, plus_code, dms, mgrs, iso6346, hs_code, unlocode), technology +11 (ulid, tsid, snowflake_id, aws_arn, s3_uri, jwt, docker_ref, git_sha, cidr, urn, data_uri), identity +15 (icd10, loinc, cpt, hcpcs, vin, eu_vat, ssn, ein, pan_india, abn, orcid, email_display, phone_e164, upc, isrc), finance +3 (figi, aba_routing, bsb), representation +4 (cas_number, inchi, smiles, color_hsl). (NNFT-244)
- **PII field** — `pii: Option<bool>` on Definition struct, 11 types tagged. `x-finetype-pii` in JSON Schema output. (NNFT-244.05)
- **`x-finetype-transform-ext`** — Extended transform metadata in schema output. (NNFT-244.05)

### Changed

- **Taxonomy precision cleanup** — Removed 2 low-precision integer-range types: `http_status_code` and `port` (false positives on plain integers). Renamed 7 currency amount types to format-structural names (amount_us→amount, amount_eu→amount_comma, etc.). Old names preserved as aliases. (NNFT-242, NNFT-243)
- **Duration regex** — Expanded to full ISO 8601 spec. `iso_8601_verbose` aliased to `iso_8601`. (NNFT-244.05)
- **bcp47 dedup** — Aliased to `locale_code`. (NNFT-244.02)

### Model

- **CharCNN v14** — Retrained on 250-type taxonomy (1500 samples/type, 372k total, 10 epochs, 86.6% training accuracy). (NNFT-245)
- **Sense classifier** — Retrained with 250-type category mappings (87.1% broad accuracy, 78.5% entity accuracy). (NNFT-245)
- **Model2Vec** — Refreshed type embeddings for all 250 types (750 embeddings × 128 dim). (NNFT-245)

### Accuracy

- **Profile eval: 95.7% label, 97.3% domain** (178/186 columns) on expanded eval suite with 43 new type columns. 3 new false positives from type overlaps (cpt/postal_code, hs_code/decimal_number, docker_ref/hostname). (NNFT-245)
- **Actionability eval: 99.9%** — 232,321/232,541 values transformed successfully. (NNFT-245)

## [0.6.3] - 2026-03-07

### Taxonomy

- **Taxonomy cleanup** — Removed 7 low-precision types, recategorized color types. Net: 216→209 types across 7 domains. (NNFT-233)
- **Geographic name removal** — Renamed 10 types from locale-based names to format-structural names: `eu_slash`→`dmy_slash`, `us_slash`→`mdy_slash`, `american`→`mdy_12h`, `european`→`dmy_hm`, `decimal_number_eu`→`decimal_number_comma`, plus 5 short-form date variants. (NNFT-234)

### Accuracy

- **Profile eval: 97.9% label, 98.6% domain** (143/146 columns correct) — up from 92.5% after v13 retrain. Five targeted pipeline fixes for entity/geography confusion: hardcoded geo override ignores confidence threshold, person-name hints override location predictions, 20+ entity-name header hints (company, venue, station, etc.), bare "address"→full_address, hardcoded hints apply at low confidence. (NNFT-235)
- **Actionability eval: 99.3%** — 226,951/228,512 values transformed successfully across 238 columns and 82 types.
- **CharCNN v13** — Retrained on 209-type taxonomy (1000 samples/type, 10 epochs, 88.1% training accuracy).

### Fixed

- Clippy `collapsible_str_replace` and `fmt` compatibility for Rust 1.94 CI.

## [0.6.2] - 2026-03-06

### Added

- **`DdlInfo` API** — new `finetype-core` struct and `Taxonomy::ddl_info()` method for DDL-oriented metadata extraction (broad_type, transform, format_string, format_string_alt, decompose). Foundation for schema generation tools. (NNFT-210)
- **`finetype schema-for` command** — profile a CSV/JSON file and output DuckDB `CREATE TABLE` statement with correct types and inline transformation comments. Supports `--table-name` override and `--output json` for structured schema. (NNFT-218)
- **`--output arrow` for schema-for** — exports Arrow IPC JSON schema format compatible with arrow-rs and pyarrow. Maps DuckDB types to Arrow DataTypes. (NNFT-219)
- **`x-finetype-*` extension fields in `finetype schema`** — JSON Schema output now includes `x-finetype-broad-type`, `x-finetype-transform`, `x-finetype-format-string` for programmatic DDL generation. (NNFT-220)

### Changed

- `finetype schema` output now includes DDL contract fields (x-finetype-*) alongside format strings, enabling direct SQL code generation.

## [0.6.1] - 2026-03-06

### Accuracy

- **Actionability eval: 99.7%** — expanded to cover transform-based types (Tier B: epochs, currency, JSON, numeric). Tier A (strptime formats) 96.2%, Tier B (transforms) 99.8%, combined 99.7% on 80 types across 204 columns. (NNFT-205)

### Added

- **`profile --validate`** — new CLI flag to run JSON Schema validation per column after classification. Outputs valid/invalid/null counts and validity rates. (NNFT-212)
- **Quality scores and file-level grades** — `ColumnQualityScore` with type_conforming_rate, null_rate, completeness metrics. File-level grade: A≥95%, B≥85%, C≥70%, D≥50%, F<50%. Available in JSON and markdown output. (NNFT-213)
- **`--output markdown`** — pipe-separated tables for profile and validate commands. Clean formatting suitable for GitHub issues and documentation. (NNFT-208)
- **Quarantine samples in validation reports** — up to 5 sample invalid values per column in JSON, markdown, and plain output. Helps users quickly understand validation failures without inspecting full dataset. (NNFT-214)
- **`format_string_alt` field** — new YAML field for type definitions with alternate format strings (e.g., ISO 8601 with/without fractional seconds). Wired through taxonomy JSON export (`--full --output json`) and schema output. (NNFT-203)

### Fixed

- **Currency broad_type mismatch** — `amount_us` and `amount_eu` now declare `broad_type: DECIMAL` to match transform output (previously VARCHAR). Fixes schema-for DDL generation. (NNFT-206)
- **Accounting notation support** — `amount_us` validation and generator now accept parenthesized negatives like `($1,234.56)`. (NNFT-206)
- **Transform stubs completed** — `julian_date` and `rfc_2822_ordinal` now have working DuckDB transforms and generators, eliminating dead-end definitions. (NNFT-204)

### Changed

- **Evaluation infrastructure** — eval binaries now test both strptime-based formats and SQL transform-based types, providing comprehensive actionability coverage.

## [0.6.0] - 2026-03-05

### Accuracy

- **Profile eval: 111/116 label (95.7%), 114/116 domain (98.3%)** — with CharCNN-v12 model (216 classes) and targeted pipeline fix. (NNFT-226)
- **Actionability eval: 96.2%** — 2760/2870 datetime values parse correctly. (NNFT-226)

### Added

- **Format Coverage expansion — 53 new type definitions** (163→216 types, 33% increase). (NNFT-223)
  - **40 datetime formats:** 15 timestamps (Apache CLF, syslog BSD/ISO, ctime, W3C DTF, ISO 8601 milliseconds/microseconds/date-only, RFC 3339 nano, SQL microseconds, Unix milliseconds/microseconds), 23 dates (Chinese 年月日, Korean 년월일, Japanese era 令和, dot-separated variants, slash variants with 2-digit year, month-first/day-first with leading zeros, abbreviated month, year-month, year-quarter), 2 periods (quarter, fiscal year).
  - **13 finance formats:** 11 currency (Indian lakh/crore, Swiss apostrophe, Brazilian real, Japanese yen, Chinese yuan, Korean won, Scandinavian comma, accounting parentheses negative, minor unit integer, cryptocurrency, generic symbol), 2 rates (basis points, yield percentage).
  - New YAML categories: `datetime.period.*` (span-based dates) and `finance.rate.*` (rates, not amounts).
- **CLI output format alignment** — `label` field added to JSON output, locale suffix in human-readable output. (NNFT-221)

### Changed

- **Model: CharCNN v11 → v12** — retrained on 216-type taxonomy with 212k samples (1000/type, 10 epochs, seed 42, 87.97% training accuracy). 44 types graduated from release_priority 1-2 → 3 to include in training data. (NNFT-226)
- **LabelCategoryMap expanded** — updated for 216 types: temporal 45→85, currency 16→29. New types routed to correct Sense categories for masked vote aggregation. (NNFT-225)
- **Header-hint location override (Step 7b-pre)** — when a hardcoded header hint points to a LOCATION_TYPE (country/city/state/region/continent) but the prediction is not a location type, the hint overrides directly. Catches Sense misrouting where country names get masked to temporal types. (NNFT-226)

### Known Issues

- **5 remaining misclassifications** — address→street_address (expected full_address), abbreviated_month_date→long_full_month, airports.name→city (expected full_name), npi→isbn, company→last_name (expected entity_name). Mix of CharCNN limitations and keyword-match ambiguity in header_hint.
- **multilingual.date actionability** — mixed date formats across locales; not addressable without multi-format support.

## [0.5.3] - 2026-03-04

### Accuracy

- **Profile eval: 113/116 label (97.4%), 114/116 domain (98.3%)** — recovered from 110/116 (94.8%) in v0.5.2. Five targeted pipeline fixes: Rule 17 UTC offset guard removal, rfc_2822/rfc_3339/sql_standard header hints before generic catch-all, same-category hardcoded hint override at ≤0.80 confidence, enhanced geography protection using unmasked votes at low Sense confidence, full_address header hint distinguished from street_address. (NNFT-194)
- **Actionability eval: 97.9%** — up from 95.4% in v0.5.2. rfc_2822_timestamp column now correctly classified (was misrouted to iso_8601 by generic `contains("timestamp")` catch-all). Remaining gap: multilingual.date mixed-format column (known limitation). (NNFT-194)

### Added

- **Locale Foundation — Layer 1: Validation expansion** — expanded locale-specific validation patterns across three type families. (NNFT-195, NNFT-196, NNFT-197)
  - `postal_code`: 14 → 50+ locales. Patterns sourced from Google libaddressinput and CLDR.
  - `phone_number`: 15 → 40+ locales. Patterns derived from Google libphonenumber.
  - `month_name` / `day_of_week`: 6 → 30+ locales. Validation lists from Unicode CLDR v46.0.0.
- **Locale Foundation — Layer 2: Generator expansion** — expanded synthetic training data generators to match validation coverage. (NNFT-198, NNFT-199, NNFT-200)
  - `postal_code` generator: 14 → 65 locales with format-aware random generation.
  - `phone_number` generator: catch-all countries promoted to named locales (46 total).
  - CLDR date/time patterns wired into `month_name`, `day_of_week`, and datetime generators (32 locales).
- **CI Sense model download** — `.github/scripts/download-model.sh` now fetches the Sense classifier model from HuggingFace, enabling the Sense→Sharpen pipeline in CI builds. (NNFT-202)

### Changed

- **Model: CharCNN v10 → v11** — retrained on locale-expanded training data (161k samples, 10 epochs, seed 42, 88.3% training accuracy). Expanded locale coverage in generators provides richer training signal for geography, identity, and datetime types. (NNFT-201)
- **Header hints refined** — specific rfc_2822, rfc_3339, and sql_standard timestamp hints now take priority over generic `iso_8601` catch-all. Bare "name" header no longer forces `full_name` — lets Sense + CharCNN decide. `full_address` distinguished from `street_address` via header keyword. (NNFT-194)
- **Same-category hint override** — when a curated `header_hint()` and CharCNN prediction share the same `domain.category` (e.g., `datetime.timestamp.*`), the header is authoritative — but only when model confidence ≤0.80 to avoid overriding correct high-confidence predictions. (NNFT-194)

### Fixed

- **UTC offset misclassification** — Rule 17 guard removed. The `[+-]HH:MM` pattern validator at ≥80% is sufficient; the guard requiring top CharCNN vote to be a time type was too restrictive after v11 retrain. (NNFT-194)
- **rfc_2822_timestamp misclassification** — was being matched by generic `contains("timestamp")` → `iso_8601` catch-all in `header_hint()`. Now matched by specific `rfc 2822` check first. Note: header normalization replaces underscores with spaces. (NNFT-194)
- **Geography protection enhanced** — when Sense confidence is very low (<0.30), checks unmasked CharCNN votes for location types instead of relying on masked (potentially empty) votes. Recovers correct predictions when Sense misroutes columns. (NNFT-194)
- **Eval manifest GT correction** — `sports_events.venue` ground truth corrected from "name" to "entity name". Venue names (stadiums, arenas) are entities, not person names. (NNFT-194)

### Known Issues

- **3 remaining misclassifications** — countries.name (→region, correct domain), world_cities.name (→full_name, Sense misroute), sports_events.venue (→city, expected entity_name). All require model retrain to resolve — CharCNN cannot distinguish geography subtypes from person names via character patterns alone.
- **multilingual.date actionability** — 60 values, 0% parse rate. Mixed date formats across locales; not addressable without multi-format support.

## [0.5.2] - 2026-03-04

### Accuracy

- **Actionability eval: 98.7%** — 2990/3030 datetime values parse via `TRY_STRPTIME`. Up from 96.0% (NNFT-191). long_full_month_date now correctly classified. (NNFT-192)
- **Profile eval: 110/116 label (94.8%), 110/116 domain (94.8%)** — regressed from 117/119 (98.3%) due to CharCNN v10 retrain boundary shifts. 6 misclassifications (utc_offset→excel_format, ean→credit_card_number, 3× name disambiguation, countries.name→full_name). Root cause: model retraining, not logic changes. Follow-up investigation planned for v0.5.3. (NNFT-192)

### Changed

- **Taxonomy: 164 → 163 types** — two removals, one addition. Net -1. (NNFT-192)
  - Removed `geography.address.street_number` — validation pattern indistinguishable from `integer_number`, causing false positives on plain numeric columns. Demotion rules in column.rs cleaned up.
  - Removed `identity.person.age` — `CAST(col AS SMALLINT)` identical to `integer_number`. 205 SOTAB false positives at 0.995 confidence. Resolves NNFT-135 entirely.
  - Added `representation.identifier.numeric_code` — all-digit VARCHAR codes with leading zeros and consistent length (ISO country numeric 840/036, NAICS, SIC, FIPS, product codes). Preserves leading zeros where integer cast would lose data. Addresses #2 analyst frustration from taxonomy revision research. (NNFT-192)

- **Model: CharCNN v9 → v10** — retrained on 163-type taxonomy. 161k samples (priority ≥1), 5 epochs, seed 42, 83.6% training accuracy. Model2Vec type embeddings regenerated (489 rows = 163 × 3 FPS). Default symlink updated. (NNFT-192)

### Fixed

- **Sense LabelCategoryMap** — updated for removed (street_number, age) and added (numeric_code) labels. (NNFT-192)
- **Measurement type detection** — only height/weight remain in MEASUREMENT_TYPES; age removed (NNFT-192).
- **Numeric attractor demotion** — street_number rules eliminated; postal_code remains only numeric attractor (NNFT-192).

### Known Issues

- **Profile eval regression under investigation** — 6 misclassifications after v10 retrain. Deferred to v0.5.3 follow-up task for accuracy recovery.

## [0.5.1] - 2026-03-03

### Accuracy

- **Profile eval: 98.3% label (117/119), 100% domain (119/119)** — up from 96.7% (116/120). Six new disambiguation mechanisms: validation-based candidate elimination (JSON Schema contracts reject impossible types), Rule 19 (percentage without '%' → decimal_number), expanded header hints (timezone, publisher, measurement keywords), hardcoded hint priority over Model2Vec, same-domain geo override, geography rescue from unmasked votes. (NNFT-188)
- **Actionability eval: 96.0%** — 2910/3030 datetime values parse successfully via `TRY_STRPTIME`. Improved from 92.7% via `format_string_alt` support for ISO 8601 fractional seconds. (NNFT-191)

### Added

- **Finance domain** — 16 new types: IBAN, SWIFT/BIC, ISIN, CUSIP, SEDOL, LEI, ISO 4217 currency codes, currency symbols, currency amounts, and more. (NNFT-177, NNFT-178)
- **Identifier category** — alphanumeric_code, html_content, locale_number added to taxonomy. (NNFT-179, NNFT-180)
- **Pure Rust ML training** — `finetype-train` crate with 4 binaries: `train-sense-model`, `train-entity-classifier`, `prepare-sense-data`, `prepare-model2vec`. All training via Candle, zero Python dependencies. Dual-format `SenseClassifier` supports both Python-trained (MHA) and Rust-trained (simple attention) models. (NNFT-185)
- **Validation-based candidate elimination** — after vote aggregation, validates top candidates against JSON Schema contracts. Eliminates candidates where >50% of sample values fail validation. (NNFT-188)
- **Rule 19: percentage demotion** — percentage winner with no '%' in values → decimal_number. (NNFT-188)
- **Geography rescue** — recovers location types from unmasked CharCNN votes when Sense misroutes location columns. (NNFT-188)
- **`format_string_alt` taxonomy field** — alternative format strings for types with common variants (e.g., ISO 8601 with optional fractional seconds). Eval tries multiple format strings per type. (NNFT-191)

### Changed

- **Taxonomy: 163 → 164 types, 6 → 7 domains** — net +3 types (IBAN, currency amounts, html_content, locale_number, alphanumeric_code added; cvv, century, screen_size, ram_size removed). New finance domain with 16 types split from identity. (NNFT-177, NNFT-178, NNFT-179, NNFT-180)
- **CharCNN v9 model** — retrained on clean 164-type taxonomy (1,000 samples/type). Refreshed Model2Vec type embeddings, Sense + Entity classifiers. `remap_collapsed_label()` eliminated — models now natively produce 164-class outputs. (NNFT-181)
- **Header hints expanded** — timezone, publisher, measurement keywords. Hardcoded hints now take priority over Model2Vec semantic hints. (NNFT-188)

### Removed

- **Python training scripts** — 11 Python files removed. All training migrated to `finetype-train` Rust crate. (NNFT-186)
- **`remap_collapsed_label()`** — no longer needed; models trained on clean 164-type taxonomy. (NNFT-181)

## [0.5.0] - 2026-03-01

### Accuracy

- **Sense & Sharpen pipeline** — two-stage column classification. Model2Vec cross-attention predicts broad category (temporal/numeric/geographic/entity/format/text) + entity subtype, then CharCNN votes are masked to category-eligible labels. Safety valve falls back to unmasked when confidence is low. 116/120 label (96.7%), 120/120 domain (100%), 0 regressions vs legacy. (NNFT-163–173)
- **Taxonomy consolidation** — collapsed 8 niche types (171→163) with backward-compatible `remap_collapsed_label()`. Zero regressions. (NNFT-162)

### Added

- **`SenseClassifier`** — Candle port of Architecture A (cross-attention over Model2Vec). 6 broad categories + 4 entity subtypes. ~3.6ms/column. (NNFT-168)
- **`Model2VecResources`** — shared tokenizer/embedding loading across Sense, semantic hints, and entity classifier. Net memory increase: 1.4MB (Sense weights only). (NNFT-165–167)
- **`LabelCategoryMap`** — maps all 163 types to Sense categories for output masking. (NNFT-169)
- **Snapshot learning** — auto-backup before model overwrite, `--seed N` for deterministic training, `manifest.json` provenance. (NNFT-146)
- **`--sharp-only` CLI flag** — opt into legacy tiered-only pipeline (disables Sense). (NNFT-173)
- **A/B evaluation infrastructure** — `eval/eval_output/sense_ab_diff.json` comparing Sense vs legacy per-column. (NNFT-172)

### Changed

- Default CLI pipeline: Sense→Sharpen replaces direct tiered cascade. Falls back to tiered when Sense model absent. (NNFT-170)
- Taxonomy: 171 → 163 types. 8 niche types collapsed. (NNFT-162)
- Profile eval expanded: 116/120 label (96.7%), 120/120 domain (100%). (NNFT-173)
- Test suite: 388 tests (7 core + 98 model + 252 CLI + 31 DuckDB). (was 187 at v0.1.0)

### Fixed

- Model2Vec `encode_batch` L2-normalisation mismatch — batch path now matches individual encoding. (NNFT-173)
- Geography protection fall-through in Sense pipeline — person-name hints no longer block general hint logic. (NNFT-173)
- Coordinate disambiguation guard — only fires when coordinate labels have competitive vote share (≥1/3 of top). (NNFT-173)

## [0.4.0] - 2026-02-27

### Accuracy

- **Entity classifier integration** — Deep Sets MLP classifies columns as person/organization/place/creative_work using Model2Vec value embeddings. When CharCNN votes full_name but column values are non-person entities, demotes to entity_name. Fires as Rule 18 between disambiguation and header hints. Entity demotion guard prevents header hints from overriding data-driven decisions. SOTAB domain: +3.9pp (64.4% → 68.3%), 3,027 columns affected (18.1%). Profile eval unchanged at 113/120 (NNFT-150, NNFT-151, NNFT-152)
- **Phone validation precision overhaul** — Established Precision Principle: for locale-specific types, only locale-confirmed validation gates confidence signals. Universal validation can reject but cannot confirm. Expanded phone locale patterns with extension suffixes, (0) trunk prefix, ZA locale, slash/en-dash separators. Telephone cardinality demotions: 254 → 24. SOTAB label: +3.0pp (39.5% → 42.5%) (NNFT-132, NNFT-136)
- **Text length demotion (Rule 16)** — full_address predictions with median value length >100 demoted to sentence. 441 columns corrected. SOTAB domain: +1.8pp (62.6% → 64.4%) (NNFT-134)
- **Duration/TLD disambiguation (Rule 14)** — SEDOL override when ≥50% of values match ISO 8601 duration pattern. TLD added to CODE_ATTRACTORS. SOTAB label: +9.0pp (30.5% → 39.5%), domain: +4.7pp (54.8% → 59.5%) (NNFT-131)
- **UTC offset override (Rule 17)** — when ≥80% of values match `[+-]HH:MM` pattern, overrides time predictions to datetime.offset.utc. Distinguishes offsets from plain time values by mandatory leading sign (NNFT-143)

### Added

- **CLI `schema` command** — export JSON Schema for any type, supports glob patterns. `taxonomy --full --output json` exports all 19 fields per type (NNFT-149)
- **Entity name and paragraph types** — `representation.text.entity_name` and `representation.text.paragraph` added to taxonomy (171 total). Addresses full_name overcall on non-person entities (NNFT-137)
- **Post-hoc locale detection** — after type classification, runs sample values against `validation_by_locale` patterns. Returns locale with highest pass rate above 50%. CLI JSON output includes `"locale"` field. Works for phone_number (15 locales) and postal_code (14 locales) (NNFT-140)
- **Expanded locale validation** — added 36 additional locale patterns for day_of_week and month_name (6 locales each). Locale detection re-runs after header hint changes (NNFT-141, NNFT-136)
- **Designation-aware is_generic** — four additive signals: attractor-demoted, boolean, hardcoded list, and taxonomy designation (broad_words/broad_characters/broad_numbers/broad_object). Hardcoded list always applies; designation expands the set further (NNFT-139)
- **Richer designation metadata** — added `broad_words`, `broad_characters`, `broad_numbers`, `broad_object` designations to taxonomy definitions for disambiguation confidence gating (NNFT-139)

### Changed

- **Profile eval expanded** — 74 → 120 columns across 21 datasets. 8 new datetime types, improved coverage for geography, identity, and measurement columns. Current: 113/120 label (94.2%), 114/120 domain (95.0%) (NNFT-148)
- **Evaluation package** — precision per type (🟢≥95%, 🟡80-95%, 🔴<80%), actionability eval (98.7% TRY_STRPTIME success), confidence calibration, overcall analysis for 10 high-risk types. Unified `make eval-report` dashboard (NNFT-147)
- **CLI batch mode** — `finetype infer --mode column --batch` reads JSONL for bulk column classification. Python eval scripts pipe benchmark columns through CLI for SOTAB/GitTables scoring (NNFT-130)
- **Retraining regression fix** — restored v0.3.0 models from HuggingFace after non-deterministic retraining caused world_cities.name regression. Snapshot learning safeguards planned (NNFT-143, NNFT-146)

## [0.3.0] - 2026-02-25

### Accuracy

- **Geography-aware header hint** — when Model2Vec maps a "name" column to full_name, new geography protection checks prevent overriding correct location predictions. Two cases: (1) keep model prediction when it's already a location type, (2) rescue attractor-demoted predictions when geography votes exist. Fixes world_cities.name → city. Profile eval 68/74 → 69/74 (NNFT-127)
- **Measurement disambiguation** — age, height, and weight are numerically indistinguishable (small integers in overlapping ranges). When the header provides a specific measurement hint but the model predicts a different measurement type, the header now wins. Fixes medical_records.height_in → height. Profile eval 69/74 → 70/74 (NNFT-128)

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
