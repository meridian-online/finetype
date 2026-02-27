# FineType

FineType is a type inference engine that detects and classifies data types in tabular datasets. It's the core analytical engine of the Noon project.

## The Noon Pillars

Every decision in this repo should reflect these principles:

1. **Spark joy for analysts** — Type inference should feel magical, not tedious. Clear output, helpful error messages, sensible defaults.
2. **Write programs that do one thing and do it well** — FineType infers types. It doesn't validate, transform, or visualise. Those are separate concerns for separate tools.
3. **Design for the future, for it will be here sooner than you think** — The type taxonomy, model architecture, and extension interfaces should accommodate new data types and formats without breaking existing behaviour.

### Precision Principle

Precision is what makes FineType valuable as a tool. A broad pattern that matches everything validates nothing. Every validation pattern, locale rule, and disambiguation heuristic must meaningfully distinguish "is this type" from "is not this type."

Concretely:
- **Prefer precise locale-specific validation over permissive universal patterns.** There is no "universal phone number" — there is E.164, and there are locale-specific formats. There is no "universal postal code" — there are country-level formats. If a type is `designation: locale_specific`, its real validation lives in `validation_by_locale`, not the universal `validation` block.
- **A validation that confirms 90% of random input is not a validation.** It's a format hint at best. Don't let it influence disambiguation decisions as if it were evidence.
- **Expanding locale coverage is the path to accuracy**, not relaxing heuristics. Each new locale pattern is a precise, measurable, testable improvement. Weakening a gate to compensate for thin coverage is borrowing against future correctness.

## Current State

**Version:** 0.3.0 (latest tag: `v0.3.0`)
**Taxonomy:** 171 definitions across 6 domains — all generators pass, 100% alignment
**Default model:** tiered-v2 (CLI) + Model2Vec semantic hints, char-cnn-v7 flat (DuckDB extension)
**Codebase:** ~20k lines of Rust across 4 crates
**CI status:** All checks pass (fmt, clippy, test, taxonomy check, smoke tests)
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged)

### Recent milestones

- **NNFT-143 retraining regression fix** (NNFT-143) — Added Rule 17: UTC offset disambiguation override (leading +/- sign distinguishes offsets from HH:MM times). Retraining to fix world_cities.name failed (non-deterministic training, no seed support) — models restored to v0.3.0 from HuggingFace. Profile eval with v0.3.0 models + current code: 69/74 (93.2%). world_cities.name regression from post-v0.3.0 code changes (geography protection only guards full_name hints, not last_name from Model2Vec). Investigation deferred to NNFT-145.
- **Entity name & paragraph taxonomy expansion** (NNFT-137) — Added `representation.text.entity_name` and `representation.text.paragraph` types to address full_name overcall on non-person entities (companies, venues, products). Types are in taxonomy (171 total) and generators, but v0.3.0 models (169 types) remain active — NNFT-137 retraining was reverted. Eval SQL updated with entity_name↔full_name interchangeability for "name" GT labels.
- **Text overcall investigation** (NNFT-134) — Root cause analysis of 5,243 full_name/full_address overcall columns (86% false positive rate). Added text length demotion rule (Rule 16): full_address predictions with median value length >100 demoted to representation.text.sentence. 441 columns corrected. SOTAB domain: 62.6% → 64.4% (+1.8pp). Finding: full_name overcall (3,086 cols) needs model retraining — no surgical rule available.
- **Phone validation precision & locale expansion** (NNFT-132, NNFT-136) — Established Precision Principle: locale-only confirmation for locale-specific types. Expanded phone locale patterns with extension suffixes, (0) trunk prefix, ZA locale, slash/en-dash separators. SOTAB format-detectable: 39.5% → 42.5% label (+3.0pp), 59.5% → 62.6% domain (+3.1pp). Telephone cardinality demotions: 254 → 24. Profile eval unchanged at 70/74.
- **Post-v0.3.0 disambiguation sprint** (NNFT-131) — Duration vs SEDOL override rule (Rule 14), TLD added to CODE_ATTRACTORS, SOTAB schema mapping expanded for DateTime/Date variants. SOTAB format-detectable: 30.5% → 39.5% label (+9.0pp), 54.8% → 59.5% domain (+4.7pp). Profile eval unchanged at 70/74.
- **v0.3.0** — Accuracy release: geography-aware header hints (NNFT-127) and measurement disambiguation (NNFT-128). Profile eval 68/74 → 70/74 (94.6%). world_cities.name now correctly predicts city; medical_records.height_in now correctly predicts height.
- **v0.2.2** — Locale-aware phone number validation (NNFT-121) with 14 locale patterns derived from libphonenumber. phone_number added to TEXT_ATTRACTORS for demotion of false positives. Infrastructure hardening, no eval score change (68/74).
- **v0.2.1** — Locale-aware postal code validation (NNFT-118), max-sim semantic matching with K=3 FPS representatives (NNFT-124), threshold tuning (NNFT-122), targeted synonyms (NNFT-123). Smarter column classification with reduced false positives.
- **v0.2.0** — Multi-signal attractor demotion (NNFT-115), JSON Schema validation engine (NNFT-116), numeric range validation (NNFT-117). Reduces false positives on generic numeric data and modernises the validation engine.
- **v0.1.9** — Model2Vec semantic column name classifier (NNFT-110), unified column-level disambiguation (NNFT-109). Profile eval 55/74 → 68/74 format-detectable correct (+13, 0 regressions). Homebrew tap auto-updated.
- **v0.1.8** — 30x tiered inference throughput, accuracy 72.6% -> 92.9% on profile eval, Windows release target, header-hint override system
- **v0.1.7** — Tiered model graph as default inference engine, `ValueClassifier` trait for polymorphic dispatch
- **v0.1.6** — CharCNN v7, evaluation infrastructure, GitTables/SOTAB benchmarks
- **DuckDB extension v0.2.0** — Tiered model, 168 types, 19 new DuckDB type mappings. Merged into community extensions (NNFT-092)

### What's in progress

- **Next accuracy targets** — 5 misses at 69/74: countries.name (intractable without cross-column context), books_catalog.publisher (city overcall, GT expects full_name), tech_systems.server_hostname (hostname prediction, GT expects full_name), people_directory.company (categorical, GT expects full_name — needs entity_name model), world_cities.name (last_name overcall — geography protection doesn't guard Model2Vec last_name hints, investigation in NNFT-145). **Model state:** v0.3.0 models (169 types) restored from HuggingFace; taxonomy has 171 types (entity_name + paragraph added but not in model). Rule 17 (UTC offset override) is the only post-v0.3.0 disambiguation rule addition. CLDR date/time patterns and 4-level locale labels (NNFT-126) are next infrastructure pieces.
- **Evaluation methodology** — NNFT-144 (discovery): investigate whether profile eval (74-column smoke test) and real-world benchmarks (GitTables 47%, SOTAB 42%) meaningfully measure type inference quality. Time-boxed 4-6 hours.

## Architecture

### Workspace layout

```
finetype/
  crates/
    finetype-core/     # Taxonomy, generators, validation, tokenizer
    finetype-model/    # CharCNN, tiered classifier, column disambiguation, training
    finetype-cli/      # CLI binary (infer, profile, generate, check, train)
    finetype-duckdb/   # DuckDB loadable extension (scalar functions)
  labels/              # Taxonomy YAML definitions (6 domain files)
  models/              # Pre-trained model directories (char-cnn-v1..v7, tiered-v1..v2, model2vec)
  eval/                # Evaluation infrastructure (GitTables, SOTAB, profile)
  tests/               # CLI smoke tests
  docs/                # Taxonomy comparison, architecture docs
  data/                # Reference data files + locale data sources (data/cldr/)
```

### Crate dependency graph

```
finetype-core  (no internal deps — taxonomy, generators, validation)
    |
finetype-model (depends on core — CharCNN, tiered inference, column mode)
    |
    +--- finetype-cli   (depends on core + model — CLI binary)
    +--- finetype-duckdb (depends on core + model — DuckDB extension)
```

### Inference pipeline

The inference system has two modes:

**1. Value-level inference** — Single string -> type label
- `CharClassifier` (flat): Single CharCNN model, 171 classes, ~1,500 val/sec
- `TieredClassifier` (hierarchical): 46 CharCNN models in T0->T1->T2 graph (34 trained, 12 direct), ~580 val/sec, higher accuracy on ambiguous types

**2. Column-level inference** — Vector of strings -> single column type
- Runs value-level inference on each value
- Aggregates predictions via majority vote
- Applies disambiguation rules (date formats, coordinates, boolean subtypes, numeric ranges, categorical detection, duration override, attractor demotion, text length demotion)
- **Duration override** (Rule 14, NNFT-131): When top vote is SEDOL but ≥50% of values match ISO 8601 duration pattern (P prefix + time component letters Y/M/W/D/T/H/S), overrides to `datetime.duration.iso_8601`. Must run before attractor demotion to prevent SEDOL being demoted to `alphanumeric_id` instead of the correct `duration`.
- **Attractor demotion** (Rule 15): Demotes over-eager specific type predictions using three signals — validation schema failure (>50%), confidence threshold (<0.85 when not locale-confirmed), and cardinality mismatch (1-20 unique values for text attractors, skipped when locale-confirmed). Requires `Taxonomy` to be wired into `ColumnClassifier`. Demoted predictions are treated as generic for header hint override. Code attractors: icao_code, ndc, cusip, top_level_domain (NNFT-131). **Locale-aware validation** (NNFT-118, NNFT-132): For types with `validation_by_locale`, Signal 1 first checks all locale patterns — if any locale achieves >50% pass rate, the prediction is locale-confirmed (skips Signals 2 and 3). Universal validation can reject (Signal 1) but cannot confirm — passing universal validation alone leaves the prediction vulnerable to all signals. This prevents permissive universal patterns from giving false confidence (see Precision Principle).
- **Text length demotion** (Rule 16, NNFT-134): When top vote is `full_address` and the median non-empty value length exceeds 100 characters, demotes to `representation.text.sentence`. Real addresses have median ~23 chars; free-form text (descriptions, recipes, paragraphs) has median ~53+ chars. Threshold 100 gives 0% false demotion on evaluation data.
- **UTC offset override** (Rule 17, NNFT-143): When top vote is any `datetime.time.*` type or `datetime.timestamp.rfc_3339`, and ≥80% of non-empty values match the `[+-]HH:MM` pattern (exactly 6 chars), overrides to `datetime.offset.utc`. The mandatory leading sign (+/-) is the syntactic distinguisher from plain time values. Runs between duration override and attractor demotion.
- **Semantic header hints** (Model2Vec): embeds column name → max-sim matching against 171 types × K=3 representative embeddings → overrides generic predictions above 0.65 threshold. Falls back to hardcoded `header_hint()` when Model2Vec unavailable. **Geography protection** (NNFT-127): when hint is `full_name`, checks if model sees geography signal — keeps location predictions rather than overriding, and rescues attractor-demoted predictions when geography votes exist. **Measurement disambiguation** (NNFT-128): when both hint and prediction are measurement types (age/height/weight), trusts the header since values are numerically indistinguishable.
- **Post-hoc locale detection** (NNFT-140): After type classification and disambiguation, runs sample values against `validation_by_locale` patterns to detect the most likely locale. Returns the locale with the highest pass rate above 50%. Locale is cleared if a header hint changes the label. Works for phone_number (15 locales) and postal_code (14 locales). Implements decision-002 Option B.
- **`is_generic` determination** (NNFT-139): `is_generic_prediction()` function uses four additive signals to decide if a prediction should yield to header hints: (1) attractor-demoted → always generic, (2) boolean → always generic, (3) hardcoded catch-all list (phone_number, first_name, iata_code, etc.) → always generic, (4) taxonomy designation `broad_words`/`broad_characters`/`broad_numbers`/`broad_object` → additionally generic. Signals are additive — hardcoded list always applies, designation expands the set further.

Both classifiers implement the `ValueClassifier` trait for polymorphic dispatch.

### Tiered model architecture

```
Tier 0 (root): DuckDB-type router (VARCHAR, BIGINT, DOUBLE, DATE, etc.)
  |
Tier 1: Domain routers per DuckDB type (e.g., VARCHAR -> address/code/person/internet/...)
  |
Tier 2: Leaf classifiers per domain (e.g., VARCHAR_person -> email/full_name/username/...)
```

- 34 specialised CharCNN models, each trained on its tier's subset
- `tier_graph.json` defines the routing hierarchy
- `manifest.txt` lists all model files for embedding
- Models stored in `models/tiered-v2/` with subdirectories per tier node

### Taxonomy structure

Labels follow `domain.category.type` hierarchy (e.g., `identity.person.email`, `datetime.date.iso`).

**6 domains:**
- `container` — JSON, XML, YAML, CSV, arrays (11 types, recursive inference)
- `datetime` — Date, time, timestamp formats across locales (46 types)
- `geography` — Addresses, coordinates, country/region codes (16 types)
- `identity` — Person, organisation, financial, medical identifiers (35 types)
- `representation` — Boolean, categorical, ordinal, numeric, text, alphanumeric (29 types)
- `technology` — Internet, development, cryptographic, file types (34 types)

Each definition in `labels/definitions_*.yaml` is a **transformation contract** specifying:
- `broad_type` — Target DuckDB type
- `format_string` — DuckDB strptime format (if date/time)
- `transform` — DuckDB SQL expression (`{col}` placeholder)
- `validation` — Pattern or constraint for the type
- `tier` — Path in the inference graph
- `decompose` — Optional struct expansion

### DuckDB extension functions

| Function | Signature | Purpose |
|---|---|---|
| `finetype(col)` | `VARCHAR -> VARCHAR` | Column-level classification (uses chunk as sample) |
| `finetype(list, header?)` | `LIST<VARCHAR>[, VARCHAR] -> VARCHAR` | Explicit column classification with optional header hint |
| `finetype_detail(col)` | `VARCHAR -> VARCHAR (JSON)` | Full classification detail (confidence, votes, DuckDB type) |
| `finetype_detail(list, header?)` | `LIST<VARCHAR>[, VARCHAR] -> VARCHAR (JSON)` | Explicit column detail |
| `finetype_cast(value)` | `VARCHAR -> VARCHAR` | Normalize value for safe TRY_CAST |
| `finetype_unpack(json)` | `VARCHAR -> VARCHAR (JSON)` | Recursively classify JSON fields |
| `finetype_version()` | `-> VARCHAR` | Extension version string |

**Note:** The DuckDB extension currently embeds the flat CharCNN model (not tiered) for performance. The `finetype()` scalar function uses chunk-aware column classification — each ~2048-row processing chunk is treated as a column sample for disambiguation.

### CLI commands

| Command | Purpose |
|---|---|
| `finetype infer` | Classify values from stdin (single or column mode). `--header` adds header hint, `--batch` reads JSONL for bulk column classification. |
| `finetype profile <file>` | Profile all columns in a CSV/Parquet file |
| `finetype check` | Validate taxonomy <-> generator alignment |
| `finetype generate` | Generate synthetic training data |
| `finetype train` | Train CharCNN models (flat or tiered) |
| `finetype taxonomy` | Print taxonomy summary |

### Evaluation infrastructure

Six evaluation components, all in `eval/`:

**Core benchmarks:**
1. **Profile eval** (`eval/profile_eval.sh`) — Regression smoke test on 74 annotated columns from 20 datasets. Scores against `schema_mapping.yaml`. Current: 93.2% label accuracy (69/74), 93.2% domain accuracy (69/74) on format-detectable (direct+close) types.
2. **GitTables 1M** (`eval/gittables/`) — Large-scale benchmark against GitTables corpus. v0.3.0 CLI: 47.1% label / 56.5% domain accuracy on format-detectable types (4,481 columns, 45,428 total). v0.1.8 DuckDB: 57.8% domain (14,850 tables, 2.7M values).
3. **SOTAB CTA** (`eval/sotab/`) — Schema.org type annotation benchmark. Post-NNFT-134 CLI: 42.5% label / 64.4% domain accuracy on format-detectable types (11,484 columns, 16,765 total). Post-NNFT-131: 39.5% label / 59.5% domain. v0.3.0 baseline: 30.5% label / 54.8% domain. v0.1.8 DuckDB: 53.7% domain (5,728 tables, 16,765 columns).

**Analyst-centric metrics (NNFT-147):**
4. **Precision per type** — SQL sections in SOTAB/GitTables eval scripts. Per-predicted-type precision with thresholds: 🟢≥95% (analyst can act), 🟡80-95% (spot-check), 🔴<80% (untrustworthy). Includes overcall analysis for 10 high-risk types (full_name, entity_name, full_address, URL, geography, postal_code) showing GT label breakdown of false positives.
5. **Actionability eval** (`eval/eval_actionability.py`) — Tests whether FineType's `format_string` predictions actually parse real data. Runs TRY_STRPTIME via DuckDB on profile eval datasets. Current: 98.3% overall (2350/2390 values), 17/18 columns at 100%. Target: >95% for datetime types.
6. **Confidence calibration** — SQL sections in SOTAB/GitTables eval scripts. Bins predictions by confidence decile, compares actual accuracy vs reported confidence. Target: calibration gap <10pp.

**Dashboard:** `make eval-report` runs profile eval + actionability eval and generates `eval/eval_output/report.md` — a unified markdown dashboard with headline metrics, precision per type, actionability by format, and evaluation component status.

All eval pipelines use `eval/config.env` for dataset paths with `envsubst` substitution in SQL templates. CLI-based eval pipelines (`eval-1m-cli`, `eval-sotab-cli`) use Python scripts to pipe columns through `finetype infer --mode column --batch`, then score with adapted SQL.

## Priority Order

Current priorities, in order:

1. **Accuracy lift** — Address top misclassification patterns, expand disambiguation rules (NNFT-090, NNFT-099, NNFT-100)
2. **Documentation** — README update with tiered architecture (NNFT-096), CHANGELOG maintenance (NNFT-095)
3. **Distribution** — Homebrew tap update (NNFT-086), crates.io keep current (NNFT-093 done)
4. **Training data quality** — Name diversity (NNFT-066), phone formats (NNFT-055), address locales (NNFT-056)
5. **New domains** — Medical identifiers (NNFT-053), SI-prefix numbers (NNFT-057), CLDR locale data (NNFT-058/060)

## Decided Items

Key architectural decisions that should not be revisited without good reason:

1. **Tiered model as default** — The T0->T1->T2 hierarchical model is the default for CLI inference. Flat model remains available via `--model-type flat` and is used in the DuckDB extension for throughput. (NNFT-084, NNFT-087, NNFT-089)

2. **Taxonomy label format: `domain.category.type`** — Three-level dotted hierarchy. Locale is a field in the YAML definition, not part of the label. (NNFT-001)

3. **YAML transformation contracts** — Each type definition specifies its DuckDB broad_type, transform SQL, and validation pattern. This is the interface between FineType and downstream tools. (NNFT-001)

4. **CharCNN architecture** — Character-level CNN for text classification. Candle (Rust) for both training and inference. No Python dependency at runtime. (NNFT-003)

5. **Column-mode disambiguation** — Majority vote + rule-based disambiguation. Rules are hardcoded in `column.rs`, not learned. Header hints override generic predictions. Two specialised hint guards: geography protection prevents `full_name` hints from overriding correct location predictions (NNFT-127), and measurement disambiguation trusts headers over model predictions when both are in the {age, height, weight} group (NNFT-128). (NNFT-065, NNFT-091, NNFT-102, NNFT-127, NNFT-128)

5a. **Model2Vec semantic header hints with max-sim matching** — Column name classification uses Model2Vec static embeddings (potion-base-4M, 7.4MB float16) with max-sim matching against pre-computed type embeddings. Each type stores K=3 representative embeddings selected via Farthest Point Sampling (FPS), avoiding centroid dilution from mean-pooling diverse synonyms. `type_embeddings.safetensors` uses interleaved layout `[n_types*K, embed_dim]`; K is inferred at load time from shape ratio (`type_embeddings.rows / label_index.len()`), so K=1 old artifacts are backward-compatible. Threshold 0.65 balances precision and recall with one known borderline FP (data→form_data at 0.687). Falls back to hardcoded `header_hint()` when Model2Vec unavailable. Model artifacts in `models/model2vec/`, embedded at build time. `prepare_model2vec.py` supports `--max-k N` (default 3) and `--legacy` (force K=1 mean-pool). No new Rust dependencies — uses existing candle-core + tokenizers. (NNFT-110, NNFT-122, NNFT-124)

6. **DuckDB extension uses flat model** — Embedding 34 tiered models is feasible (11MB binary) but the flat model is simpler and faster for batch SQL workloads. The extension uses chunk-aware column classification instead. (NNFT-092)

7. **Models on HuggingFace** — Pre-trained models hosted at `hughcameron/finetype` on HuggingFace. CI downloads models via `.github/scripts/download-model.sh`. Models are not committed to the git repo. (NNFT-020, NNFT-088)

8. **Boolean taxonomy restructured** — Moved from `technology.development.boolean` to `representation.boolean.{binary,initials,terms}` for semantic clarity. (NNFT-075)

9. **Pre-commit hook in `.githooks/`** — Runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`. Activated via `make setup`. (NNFT-072)

10. **Evaluation paths via config.env** — All eval scripts use `eval/config.env` + `envsubst` for dataset paths. No hardcoded absolute paths. (NNFT-108)

11. **Attractor demotion (Rule 15)** — Multi-signal disambiguation rule that demotes over-eager specific type predictions (postal_code, cvv, first_name, icao_code, top_level_domain, etc.) to generic `representation.*` types. Three signals: validation failure (>50% fail type's regex), confidence threshold (<0.85 when not locale-confirmed), cardinality mismatch (1-20 unique values for text attractors, skipped when locale-confirmed). For locale-specific types, only `locale_confirmed` (from `validation_by_locale`) gates Signals 2 and 3 — universal validation can reject but cannot confirm (see Decided Item 14). Taxonomy is wired into `ColumnClassifier` via `set_taxonomy()`. Demoted predictions treated as generic for header hint override. `full_name` deliberately excluded from attractor list — too many legitimate uses. TLD added to CODE_ATTRACTORS (NNFT-131) — numeric values fail TLD's alphabetic validation pattern. (NNFT-115, NNFT-131, NNFT-132)

11a. **Duration override (Rule 14)** — When top vote is `identity.payment.sedol` and ≥50% of non-empty values start with 'P' and contain ISO 8601 time component letters (Y/M/W/D/T/H/S), overrides to `datetime.duration.iso_8601`. Runs before attractor demotion because SEDOL is in CODE_ATTRACTORS — without this rule, attractor demotion would demote to `alphanumeric_id` instead of the correct `duration`. Pattern handles both standard (PT20M, P1DT12H) and malformed (PD1TH0M0) durations found in SOTAB. (NNFT-131)

12. **JSON Schema validation via jsonschema-rs** — Validation uses `jsonschema` crate (v0.42.1, pure Rust, MIT, Draft 2020-12) instead of hand-rolled regex. `CompiledValidator` pre-compiles schemas once; taxonomy caches validators via `compile_validators()`. Hybrid strategy: string keywords delegated to jsonschema, numeric bounds (minimum/maximum) handled manually for string→f64 parsing semantics. `Taxonomy::clone()` drops the cache (jsonschema::Validator doesn't impl Clone). Enables future `format`, `oneOf`, `if/then` keywords. (NNFT-116)

13. **Locale-specific validation via `validation_by_locale`** — Taxonomy definitions can include per-locale validation schemas alongside the universal `validation` block. `compile_locale_validators()` pre-compiles locale patterns into a nested cache (label → locale → CompiledValidator). Attractor demotion Signal 1 checks locale patterns first — if any locale achieves >50% pass rate on sample values, the prediction is locale-confirmed (skips demotion). Currently used for 5 types: postal_code (14 locales, Google libaddressinput Apache 2.0), phone_number (15 locales, Google libphonenumber Apache 2.0), calling_code (17 locales, ITU-T E.164), month_name (6 locales, Unicode CLDR), day_of_week (6 locales, Unicode CLDR). Phone patterns include optional extension suffixes, (0) trunk prefix notation, and expanded separators. Patterns embedded in YAML, not downloaded at runtime. (NNFT-118, NNFT-121, NNFT-136, NNFT-141)

14. **Validation precision for locale-specific types** — For types marked `designation: locale_specific`, validation has three tiers with distinct semantics. (1) **Locale validation** (`validation_by_locale`): the real confirmation — locale-specific structural patterns (digit counts, grouping rules per country). Sets `locale_confirmed`. (2) **Universal validation** (`validation`): a necessary format check that can reject (Signal 1 demotion) but cannot confirm. Passing universal validation alone means "format-compatible but unconfirmed." (3) **No match**: demote. Only `locale_confirmed` gates Signals 2 and 3 for locale-specific types. Universal validation success without locale confirmation provides no special treatment. This prevents permissive universal patterns (e.g., phone's `^[+]?[0-9\s()\-\.]+$`) from giving false confidence. The path to accuracy is expanding locale coverage, not relaxing gates. (NNFT-132)

15. **Designation-aware `is_generic` determination** — `is_generic_prediction()` function replaces the old inline match block. Uses four additive signals: (1) attractor-demoted → always generic, (2) boolean → always generic, (3) hardcoded catch-all list → always generic, (4) taxonomy designation `broad_words`/`broad_characters`/`broad_numbers`/`broad_object` → additionally generic. **Critical**: hardcoded list (Signal 3) runs before taxonomy lookup (Signal 4) — designation is additive, never removes types from the generic set. This prevents types like `phone_number` (locale_specific designation, but in hardcoded list) from losing their generic status when taxonomy is present. (NNFT-139)

16. **Post-hoc locale detection** — After type classification and all disambiguation rules, `detect_locale_from_validation()` runs sample values against `validation_by_locale` patterns for the classified type. Returns the locale with the highest pass rate above 50%. Implements decision-002 Option B: locale is a composable add-on, not a classification output. When a header hint changes the label, locale detection is re-run for the new type (NNFT-141 fix — previously locale was cleared to None). CLI JSON output includes `"locale"` field when detected. Works for all types with `validation_by_locale`. (NNFT-140, NNFT-141, decision-002)

17. **UTC offset override (Rule 17)** — When top vote is any `datetime.time.*` type or `datetime.timestamp.rfc_3339`, and ≥80% of non-empty values match `^[+-]\d{2}:\d{2}$` (exactly 6 chars), overrides to `datetime.offset.utc`. The CharCNN confuses UTC offsets like "+05:30" with time values like "14:30" because both share the HH:MM structure. The mandatory leading sign (+/-) is the syntactic distinguisher. Runs between Rule 14 (duration override) and Rule 15 (attractor demotion). (NNFT-143)

## Build & Test

```bash
# First time setup
make setup              # Install git hooks

# Development cycle
cargo build             # Build default members (core, model, cli)
cargo test              # Run test suite
cargo run -- check      # Validate taxonomy/generator alignment

# Full CI locally
make ci                 # fmt + clippy + test + check

# DuckDB extension (separate, needs model files)
cargo build -p finetype_duckdb --release

# Release build (all targets)
make build-release

# Evaluation
make eval-profile       # Profile eval (annotated CSVs)
make eval-1m            # GitTables 1M via DuckDB extension (requires corpus)
make eval-sotab         # SOTAB CTA via DuckDB extension (requires corpus)
make eval-1m-cli        # GitTables 1M via CLI batch mode (requires corpus)
make eval-sotab-cli     # SOTAB CTA via CLI batch mode (requires corpus)
make eval-actionability # Actionability eval (TRY_STRPTIME on profile data)
make eval-report        # Unified markdown dashboard (profile + actionability)
```

## Key File Reference

| What | Where |
|---|---|
| Taxonomy definitions | `labels/definitions_*.yaml` (6 files) |
| Tiered model graph | `models/tiered-v2/tier_graph.json` |
| Column disambiguation | `crates/finetype-model/src/column.rs` |
| Header hint overrides | `crates/finetype-model/src/column.rs` (search `header_hint`) |
| Locale detection | `crates/finetype-model/src/column.rs` (search `detect_locale_from_validation`) |
| Locale detection architecture | `docs/LOCALE_DETECTION_ARCHITECTURE.md` |
| Semantic hint classifier | `crates/finetype-model/src/semantic.rs` |
| Model2Vec artifacts | `models/model2vec/` (tokenizer, embeddings, type_embeddings, label_index) |
| Model2Vec prep script | `scripts/prepare_model2vec.py` |
| DuckDB type mappings | `crates/finetype-duckdb/src/type_mapping.rs` |
| Value normalization | `crates/finetype-duckdb/src/normalize.rs` |
| CLI entry point | `crates/finetype-cli/src/main.rs` |
| CI workflow | `.github/workflows/ci.yml` |
| Release workflow | `.github/workflows/release.yml` |
| Eval config | `eval/config.env` |
| Schema mapping | `eval/schema_mapping.yaml` |
| Actionability eval | `eval/eval_actionability.py` |
| Eval report generator | `eval/eval_report.py` |
| Evaluation discovery brief | `discovery/evaluation-method/BRIEF.md` |
| Smoke tests | `tests/smoke.sh` |
| Locale data attribution | `data/cldr/README.md` |

## Backlog Discipline

**Every bug fix, feature, and release MUST have a corresponding backlog task.**

This includes:

- **Bug fixes** — Create a task (status: Done if already fixed) with root cause, fix description, and affected files
- **Releases** — Tag releases should reference the backlog tasks included
- **Investigations** — Even exploratory work that produces findings gets a task
- **Infrastructure changes** — CI, build system, deployment changes

If the work is already done, create the task retroactively with status `Done`, check all ACs, and write a final summary. No exceptions — this is how we maintain an audit trail.

<!-- BACKLOG.MD MCP GUIDELINES START -->

<CRITICAL_INSTRUCTION>

## BACKLOG WORKFLOW INSTRUCTIONS

This project uses Backlog.md MCP for all task and project management activities.

**CRITICAL GUIDANCE**

- If your client supports MCP resources, read `backlog://workflow/overview` to understand when and how to use Backlog for this project.
- If your client only supports tools or the above request fails, call `backlog.get_workflow_overview()` tool to load the tool-oriented overview (it lists the matching guide tools).

- **First time working here?** Read the overview resource IMMEDIATELY to learn the workflow
- **Already familiar?** You should have the overview cached ("## Backlog.md Overview (MCP)")
- **When to read it**: BEFORE creating tasks, or when you're unsure whether to track work

These guides cover:
- Decision framework for when to create tasks
- Search-first workflow to avoid duplicates
- Links to detailed guides for task creation, execution, and finalization
- MCP tools reference

You MUST read the overview resource to understand the complete workflow. The information is NOT summarized here.

</CRITICAL_INSTRUCTION>

<!-- BACKLOG.MD MCP GUIDELINES END -->
