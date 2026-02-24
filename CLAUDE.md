# FineType

FineType is a type inference engine that detects and classifies data types in tabular datasets. It's the core analytical engine of the Noon project.

## The Noon Pillars

Every decision in this repo should reflect these principles:

1. **Spark joy for analysts** — Type inference should feel magical, not tedious. Clear output, helpful error messages, sensible defaults.
2. **Write programs that do one thing and do it well** — FineType infers types. It doesn't validate, transform, or visualise. Those are separate concerns for separate tools.
3. **Design for the future, for it will be here sooner than you think** — The type taxonomy, model architecture, and extension interfaces should accommodate new data types and formats without breaking existing behaviour.

## Current State

**Version:** 0.2.0 (latest tag: `v0.2.0`)
**Taxonomy:** 169 definitions across 6 domains — all generators pass, 100% alignment
**Default model:** tiered-v2 (CLI) + Model2Vec semantic hints, char-cnn-v7 flat (DuckDB extension)
**Codebase:** ~20k lines of Rust across 4 crates
**CI status:** All checks pass (fmt, clippy, test, taxonomy check, smoke tests)
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged)

### Recent milestones

- **v0.2.0** — Multi-signal attractor demotion (NNFT-115), JSON Schema validation engine (NNFT-116), numeric range validation (NNFT-117). Reduces false positives on generic numeric data and modernises the validation engine.
- **v0.1.9** — Model2Vec semantic column name classifier (NNFT-110), unified column-level disambiguation (NNFT-109). Profile eval 55/74 → 68/74 format-detectable correct (+13, 0 regressions). Homebrew tap auto-updated.
- **v0.1.8** — 30x tiered inference throughput, accuracy 72.6% -> 92.9% on profile eval, Windows release target, header-hint override system
- **v0.1.7** — Tiered model graph as default inference engine, `ValueClassifier` trait for polymorphic dispatch
- **v0.1.6** — CharCNN v7, evaluation infrastructure, GitTables/SOTAB benchmarks
- **DuckDB extension v0.2.0** — Tiered model, 168 types, 19 new DuckDB type mappings. Merged into community extensions (NNFT-092)

### What's in progress

- **NNFT-118** — Discovery: locale-specific type labels for postal codes, phone numbers, and addresses. Spike to evaluate whether locale-aware inference improves disambiguation accuracy.

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
  data/                # Reference data files
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
- Applies disambiguation rules (date formats, coordinates, boolean subtypes, numeric ranges, categorical detection, attractor demotion)
- **Attractor demotion** (Rule 14): Demotes over-eager specific type predictions using three signals — validation schema failure (>50%), confidence threshold (<0.85 when not validation-confirmed), and cardinality mismatch (1-20 unique values for text attractors). Requires `Taxonomy` to be wired into `ColumnClassifier`. Demoted predictions are treated as generic for header hint override.
- **Semantic header hints** (Model2Vec): embeds column name → cosine similarity against 169 pre-computed type embeddings → overrides generic predictions above 0.70 threshold. Falls back to hardcoded `header_hint()` when Model2Vec unavailable.
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
| `finetype infer` | Classify values from stdin (single or column mode) |
| `finetype profile <file>` | Profile all columns in a CSV/Parquet file |
| `finetype check` | Validate taxonomy <-> generator alignment |
| `finetype generate` | Generate synthetic training data |
| `finetype train` | Train CharCNN models (flat or tiered) |
| `finetype taxonomy` | Print taxonomy summary |

### Evaluation infrastructure

Three evaluation benchmarks, all in `eval/`:

1. **Profile eval** (`eval/profile_eval.sh`) — Runs `finetype profile` on annotated CSVs, scores against `schema_mapping.yaml`. Current: 92.9% accuracy.
2. **GitTables 1M** (`eval/gittables/`) — Large-scale benchmark against GitTables corpus. v0.1.8: 57.8% domain accuracy on format-detectable types (14,850 tables, 2.7M values).
3. **SOTAB CTA** (`eval/sotab/`) — Schema.org type annotation benchmark. v0.1.8: 53.7% domain accuracy (5,728 tables, 16,765 columns).

All eval pipelines use `eval/config.env` for dataset paths with `envsubst` substitution in SQL templates.

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

5. **Column-mode disambiguation** — Majority vote + rule-based disambiguation. Rules are hardcoded in `column.rs`, not learned. Header hints override generic predictions. (NNFT-065, NNFT-091, NNFT-102)

5a. **Model2Vec semantic header hints** — Column name classification uses Model2Vec static embeddings (potion-base-4M, 7.4MB float16) with cosine similarity against pre-computed type embeddings. Threshold 0.70 tuned for zero false positives on generics. Falls back to hardcoded `header_hint()` when Model2Vec unavailable. Model artifacts in `models/model2vec/`, embedded at build time. No new Rust dependencies — uses existing candle-core + tokenizers. (NNFT-110)

6. **DuckDB extension uses flat model** — Embedding 34 tiered models is feasible (11MB binary) but the flat model is simpler and faster for batch SQL workloads. The extension uses chunk-aware column classification instead. (NNFT-092)

7. **Models on HuggingFace** — Pre-trained models hosted at `hughcameron/finetype` on HuggingFace. CI downloads models via `.github/scripts/download-model.sh`. Models are not committed to the git repo. (NNFT-020, NNFT-088)

8. **Boolean taxonomy restructured** — Moved from `technology.development.boolean` to `representation.boolean.{binary,initials,terms}` for semantic clarity. (NNFT-075)

9. **Pre-commit hook in `.githooks/`** — Runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`. Activated via `make setup`. (NNFT-072)

10. **Evaluation paths via config.env** — All eval scripts use `eval/config.env` + `envsubst` for dataset paths. No hardcoded absolute paths. (NNFT-108)

11. **Attractor demotion (Rule 14)** — Multi-signal disambiguation rule that demotes over-eager specific type predictions (postal_code, cvv, first_name, icao_code, etc.) to generic `representation.*` types. Three signals: validation failure (>50% fail type's regex), confidence threshold (<0.85 when not validation-confirmed), cardinality mismatch (1-20 unique values for text attractors). Taxonomy is wired into `ColumnClassifier` via `set_taxonomy()`. Demoted predictions treated as generic for header hint override. `full_name` deliberately excluded from attractor list — too many legitimate uses. (NNFT-115)

12. **JSON Schema validation via jsonschema-rs** — Validation uses `jsonschema` crate (v0.42.1, pure Rust, MIT, Draft 2020-12) instead of hand-rolled regex. `CompiledValidator` pre-compiles schemas once; taxonomy caches validators via `compile_validators()`. Hybrid strategy: string keywords delegated to jsonschema, numeric bounds (minimum/maximum) handled manually for string→f64 parsing semantics. `Taxonomy::clone()` drops the cache (jsonschema::Validator doesn't impl Clone). Enables future `format`, `oneOf`, `if/then` keywords. (NNFT-116)

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
make eval-1m            # GitTables 1M (requires corpus)
make eval-sotab         # SOTAB CTA (requires corpus)
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
