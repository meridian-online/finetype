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
**Taxonomy:** 169 definitions across 6 domains — all generators pass, 100% alignment
**Default model:** tiered-v2 (CLI) + Model2Vec semantic hints, char-cnn-v7 flat (DuckDB extension)
**Codebase:** ~20k lines of Rust across 4 crates
**CI status:** All checks pass (fmt, clippy, test, taxonomy check, smoke tests)
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged)

### Recent milestones

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

- **Next accuracy targets** — 4 remaining misses at 70/74: countries.name (intractable without cross-column context), books_catalog.publisher, people_directory.company, tech_systems.server_hostname (GT mapping issue). CLDR date/time patterns and 4-level locale labels (NNFT-126) are next infrastructure pieces.

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
- `CharClassifier` (flat): Single CharCNN model, 169 classes, ~1,500 val/sec
- `TieredClassifier` (hierarchical): 34 CharCNN models in T0->T1->T2 graph, ~580 val/sec, higher accuracy on ambiguous types

**2. Column-level inference** — Vector of strings -> single column type
- Runs value-level inference on each value
- Aggregates predictions via majority vote
- Applies disambiguation rules (date formats, coordinates, boolean subtypes, numeric ranges, categorical detection, duration override, attractor demotion)
- **Duration override** (Rule 14, NNFT-131): When top vote is SEDOL but ≥50% of values match ISO 8601 duration pattern (P prefix + time component letters Y/M/W/D/T/H/S), overrides to `datetime.duration.iso_8601`. Must run before attractor demotion to prevent SEDOL being demoted to `alphanumeric_id` instead of the correct `duration`.
- **Attractor demotion** (Rule 15): Demotes over-eager specific type predictions using three signals — validation schema failure (>50%), confidence threshold (<0.85 when not locale-confirmed), and cardinality mismatch (1-20 unique values for text attractors, skipped when locale-confirmed). Requires `Taxonomy` to be wired into `ColumnClassifier`. Demoted predictions are treated as generic for header hint override. Code attractors: icao_code, ndc, cusip, top_level_domain (NNFT-131). **Locale-aware validation** (NNFT-118, NNFT-132): For types with `validation_by_locale`, Signal 1 first checks all locale patterns — if any locale achieves >50% pass rate, the prediction is locale-confirmed (skips Signals 2 and 3). Universal validation can reject (Signal 1) but cannot confirm — passing universal validation alone leaves the prediction vulnerable to all signals. This prevents permissive universal patterns from giving false confidence (see Precision Principle).
- **Semantic header hints** (Model2Vec): embeds column name → max-sim matching against 169 types × K=3 representative embeddings → overrides generic predictions above 0.65 threshold. Falls back to hardcoded `header_hint()` when Model2Vec unavailable. **Geography protection** (NNFT-127): when hint is `full_name`, checks if model sees geography signal — keeps location predictions rather than overriding, and rescues attractor-demoted predictions when geography votes exist. **Measurement disambiguation** (NNFT-128): when both hint and prediction are measurement types (age/height/weight), trusts the header since values are numerically indistinguishable.
- `is_generic` flag marks types that should yield to header hints (includes attractor-demoted predictions)

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
- `representation` — Boolean, categorical, ordinal, numeric, alphanumeric (27 types)
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

Three evaluation benchmarks, all in `eval/`:

1. **Profile eval** (`eval/profile_eval.sh`) — Runs `finetype profile` on annotated CSVs, scores against `schema_mapping.yaml`. Current: 92.9% accuracy.
2. **GitTables 1M** (`eval/gittables/`) — Large-scale benchmark against GitTables corpus. v0.3.0 CLI: 47.1% label / 56.5% domain accuracy on format-detectable types (4,481 columns, 45,428 total). v0.1.8 DuckDB: 57.8% domain (14,850 tables, 2.7M values).
3. **SOTAB CTA** (`eval/sotab/`) — Schema.org type annotation benchmark. Post-NNFT-131 CLI: 39.5% label / 59.5% domain accuracy on format-detectable types (11,484 columns, 16,765 total). v0.3.0 baseline: 30.5% label / 54.8% domain. v0.1.8 DuckDB: 53.7% domain (5,728 tables, 16,765 columns).

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

13. **Locale-specific validation via `validation_by_locale`** — Taxonomy definitions can include per-locale validation schemas alongside the universal `validation` block. `compile_locale_validators()` pre-compiles locale patterns into a nested cache (label → locale → CompiledValidator). Attractor demotion Signal 1 checks locale patterns first — if any locale achieves >50% pass rate on sample values, the prediction is locale-confirmed (skips demotion). Currently used for postal_code (14 locales sourced from Google libaddressinput, Apache 2.0) and phone_number (14 locales derived from Google libphonenumber, Apache 2.0). Patterns embedded in YAML, not downloaded at runtime. (NNFT-118, NNFT-121)

14. **Validation precision for locale-specific types** — For types marked `designation: locale_specific`, validation has three tiers with distinct semantics. (1) **Locale validation** (`validation_by_locale`): the real confirmation — locale-specific structural patterns (digit counts, grouping rules per country). Sets `locale_confirmed`. (2) **Universal validation** (`validation`): a necessary format check that can reject (Signal 1 demotion) but cannot confirm. Passing universal validation alone means "format-compatible but unconfirmed." (3) **No match**: demote. Only `locale_confirmed` gates Signals 2 and 3 for locale-specific types. Universal validation success without locale confirmation provides no special treatment. This prevents permissive universal patterns (e.g., phone's `^[+]?[0-9\s()\-\.]+$`) from giving false confidence. The path to accuracy is expanding locale coverage, not relaxing gates. (NNFT-132)

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
```

## Key File Reference

| What | Where |
|---|---|
| Taxonomy definitions | `labels/definitions_*.yaml` (6 files) |
| Tiered model graph | `models/tiered-v2/tier_graph.json` |
| Column disambiguation | `crates/finetype-model/src/column.rs` |
| Header hint overrides | `crates/finetype-model/src/column.rs` (search `header_hint`) |
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
