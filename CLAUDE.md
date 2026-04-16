# Mission

Build reliable, well-tested software through clarity of intent and rigorous verification.
Every session starts aligned on purpose. Every change ships with evidence it works.

**Values:** Clarity over ceremony. Testing over trust. Decisions captured, not forgotten.

---

# FineType

FineType is a type inference engine that detects and classifies data types in tabular datasets. It's the core analytical engine of the Meridian project.

## The Meridian Pillars

Every decision in this repo should reflect these principles:

1. **Spark joy for analysts** — Type inference should feel magical, not tedious. Clear output, helpful error messages, sensible defaults.
2. **Write programs that do one thing and do it well** — Each command has one job: `profile` discovers, `schema` generates, `validate` enforces, `load` transforms. Separate concerns for separate tools.
3. **Design for the future, for it will be here sooner than you think** — The type taxonomy, model architecture, and extension interfaces should accommodate new data types and formats without breaking existing behaviour.

### Precision Principle

Precision is what makes FineType valuable. Every validation pattern, locale rule, and disambiguation heuristic must meaningfully distinguish "is this type" from "is not this type."

- **Prefer precise locale-specific validation over permissive universal patterns.** If a type is `designation: locale_specific`, its real validation lives in `validation_by_locale`, not the universal `validation` block.
- **A validation that confirms 90% of random input is not a validation.**
- **Expanding locale coverage is the path to accuracy**, not relaxing heuristics.

## Current State

**Version:** 0.6.16
**Taxonomy:** 240 definitions across 7 domains (container: 11, datetime: 84, finance: 28, geography: 25, identity: 33, representation: 33, technology: 26) — all generators pass, 100% alignment
**Default model:** Multi-branch→Sharpen pipeline with sherlock-v13 (5-branch: char+embed+stats+header+validation). Single forward pass per column replaces ~100 CharCNN value-level inferences. Profile eval: **212/227 (93.4% label, 93.8% domain)** on 35 datasets. Legacy Sense→Sharpen path remains in code but multi-branch is the CLI/MCP/DuckDB default.
**Features:** 36-dim deterministic feature extractor, column-level aggregation (mean, variance, min, max), 6 feature-based disambiguation rules (F1–F6), 19 value-based disambiguation rules (R1–R19), Model2Vec semantic header hints, hardcoded header hints with domain-dependent thresholds (same-domain 0.95, cross-domain 0.85).
**Codebase:** ~20k lines of Rust across 9 crates (including finetype-train for pure Rust ML training, finetype-mcp for MCP server). Zero Python dependencies (build + runtime).
**CI status:** All checks pass (fmt, clippy, test, taxonomy check). 383 model tests, zero warnings.
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged), MCP server (`finetype mcp`)

### Recent work

- **v13 retrain — data quality + architecture** (m-16) — Retrained with 4 tiers of improvements from the v12 data quality audit: distilled data decontamination (removed state_code→country_code remap, dropped mislabeled SSN/user_agent rows, filtered phone/postal), new validation patterns (http_method, user_agent, latitude, geohash tightened), distilled cap at 600/type with hard-negative mining, and validation branch resize ([128,64]→[192,128]). Added `geography.location.state_code` as 240th type. Per-branch gradient norms logged to results.json. Profile eval: **201→212/227 (+11 columns, 93.4% label, 93.8% domain)**. Sprint goal exceeded. Specs: `specs/2026-04-16-v13-retrain/`, `specs/2026-04-16-v12-data-quality-audit/`.
- **v12 data quality audit** — Fixed validation branch loading bug (vb.pp prefix mismatch). Audited all 23 v12 misclassifications. Root causes: model_error(9), training_collision(8), data_gap(6). Produced retrain brief with 4 priority tiers that drove v13. Spec: `specs/2026-04-16-v12-data-quality-audit/`.
- **v11 retraining + accuracy gap closure** — Retrained multi-branch model (sherlock-v11) with 70/30 distillation-heavy mix. Expanded eval from 190 to 227 columns across 35 datasets. Audited 34 misclassifications, fixed 6 ground-truth labels, 3 broken transforms, and phantom label matches. Specs: `specs/2026-04-12-accuracy-gap-retraining/`, `specs/2026-04-12-sharpen-header-bugfixes/`.
- **Multi-branch pipeline integration** (m-15) — Replaced Sense+CharCNN with multi-branch as the default inference pipeline. Sharpen layer (feature_sharpen F1–F6, value_sharpen R1–R19, header hints) post-processes multi-branch output. Decisions 0041–0042.

### What's next

- **Publish v13 to HuggingFace** — Required for DuckDB extension runtime download. v12 was never published (superseded by v13).
- **15 remaining misclassifications** — Hierarchical subtypes (data_uri→url, email_display→email, phone_e164→phone_number), country↔country_code, user_agent→jwt, decimal_number confusion. Needs fresh audit to determine next approach.
- **Actionability format_string gaps** — Expanded eval set exposed empty format_strings on iso_8601, iso, dmy_short_dot (99.9% overall after v13). These are taxonomy definition gaps, not model issues.

### Architectural direction (settled — do not re-ask)

- **Multi-branch as Sense replacement** (decision 0041): The multi-branch model (sherlock-v4-sibling) is the default classifier. It replaces both Sense and CharCNN — single forward pass per column. The Sharpen layer (feature_sharpen, value_sharpen, header hints) post-processes multi-branch output. Rules are progressively retired as the model improves.
- **Remove regex header hints** (decision 0042): Regex-based `header_hint()` and hardcoded header rules are deprecated in favour of learned approaches — multi-branch header branch (Model2Vec), sibling-context attention, and Model2Vec semantic matching. No more regex rabbit holes.
- **Strength through simplification** (decision 0038): Prefer retraining over adding disambiguation rules. Rules are a last resort when the model demonstrably cannot learn a pattern.

## Architecture

### Workspace layout

```
finetype/
  crates/
    finetype-core/     # Taxonomy, generators, validation, tokenizer, table_validator
    finetype-model/    # CharCNN, tiered classifier, column disambiguation, training
    finetype-cli/      # CLI binary (infer, profile, generate, check, train, mcp)
    finetype-mcp/      # MCP server (rmcp v1.1.0, 8 tools, taxonomy resources)
    finetype-duckdb/   # DuckDB loadable extension (scalar functions)
    finetype-eval/     # Evaluation binaries (report, actionability, GitTables, SOTAB)
    finetype-candle-spike/  # ML training feasibility spike (Candle 0.8)
    finetype-train/    # Pure Rust ML training (Sense, Entity, data pipeline)
    finetype-build-tools/  # Build utilities (DuckDB extension metadata)
  labels/              # Taxonomy YAML definitions (7 domain files)
  models/              # Pre-trained model directories
  eval/                # Evaluation infrastructure (GitTables, SOTAB, profile)
  tests/               # CLI smoke tests
  data/                # Reference data files + locale data sources (data/cldr/)
```

### Crate dependency graph

```
finetype-core  (no internal deps — taxonomy, generators, validation)
    |
finetype-model (depends on core — multi-branch, CharCNN, column classification, Sharpen rules)
    |
    +--- finetype-cli   (depends on core + model + mcp — CLI binary)
    +--- finetype-mcp   (depends on core + model — MCP server library)
    +--- finetype-duckdb (depends on core + model — DuckDB extension)

finetype-eval  (standalone — eval binaries, depends on csv/parquet/duckdb/arrow)
```

### Inference pipeline

**Column-level (Multi-branch→Sharpen, default):** Vector of strings + header → single column type:
1. Optional sibling-context attention enriches headers with cross-column context
2. Sample 100 values, extract 4-branch features (960 char + 512 embed + 36 stats + header)
3. Multi-branch forward pass → type label + confidence (single pass per column)
4. Sharpen post-processing (no neural inference):
   - `feature_sharpen()`: F1–F6 rules on label + 36-dim ColumnFeatures
   - `value_sharpen()`: R1–R19 rules on label + values + confidence
   - `apply_header_sharpen()`: Model2Vec semantic header matching
5. Post-hoc locale detection via `validation_by_locale` patterns

**Value-level (legacy):** Single string → type label via `CharClassifier` (flat, 239 classes) or `TieredClassifier` (34 CharCNN models). Both implement `ValueClassifier` trait. Not used in the default pipeline.

Key implementation files: `column.rs` (Sharpen rules + pipeline), `semantic.rs` (header hints), `sibling_context.rs` (attention). Legacy Sense→CharCNN path exists but multi-branch is the default for CLI, MCP, and DuckDB.

### Tiered model architecture

```
Tier 0 (root): DuckDB-type router (VARCHAR, BIGINT, DOUBLE, DATE, etc.)
  → Tier 1: Domain routers (VARCHAR → address/code/person/internet/...)
    → Tier 2: Leaf classifiers (VARCHAR_person → email/full_name/username/...)
```

34 specialised CharCNN models. Graph in `models/tiered-v2/tier_graph.json`.

### Taxonomy structure

Labels: `domain.category.type` (e.g., `identity.person.email`). 7 domains: container (12), datetime (84), finance (31), geography (25), identity (34), representation (36), technology (28).

Each definition in `labels/definitions_*.yaml` specifies: `broad_type` (DuckDB type), `format_string`, `transform` (SQL expression), `validation`, `tier`, `decompose`.

### DuckDB extension

| Function | Purpose |
|---|---|
| `finetype(col)` / `finetype(list, header?)` | Column-level classification |
| `finetype_detail(col)` / `finetype_detail(list, header?)` | Full detail (JSON) |
| `finetype_cast(value)` | Normalize value for TRY_CAST |
| `finetype_unpack(json)` | Recursively classify JSON fields |
| `finetype_validate(value, schema_json)` | Schema-driven validation (returns 'valid' or error message) |
| `finetype_version()` | Version string |

Uses multi-branch model downloaded at runtime via hf_hub (cached after first download). `FINETYPE_MODEL_DIR` env var overrides with local path. Chunk-aware column classification (~2048-row chunks). `finetype_validate` uses cached schema parsing for performance.

### MCP server

`finetype mcp` starts an MCP server over stdio transport (rmcp v1.1.0). AI agents launch it as a subprocess.

**Tools (8):**

| Tool | Purpose |
|---|---|
| `infer` | Classify values (single or column mode with header) |
| `profile` | Profile all columns in CSV file (path or inline data) |
| `ddl` | Generate CREATE TABLE DDL from file profiling |
| `taxonomy` | Search/filter type taxonomy by domain/category/query |
| `schema` | Export JSON Schema — type-level (by key) or table-level (by file path/data) |
| `validate` | Schema-driven CSV validation — returns valid/invalid counts + error details |
| `generate` | Generate synthetic sample data for a type |

**Resources:** `finetype://taxonomy`, `finetype://taxonomy/{domain}`, `finetype://taxonomy/{d}.{c}.{t}`

All tools return JSON primary content + markdown summary. File tools accept `path` or inline `data`.

### CLI commands

| Command | Purpose |
|---|---|
| `finetype infer` | Classify values (single/column/batch mode) |
| `finetype profile <file>` | Profile all columns in CSV/Parquet (`-o plain\|json\|csv\|markdown\|arrow`, `--enum-threshold N`, `--verbose`) |
| `finetype check` | Validate taxonomy ↔ generator alignment |
| `finetype generate` | Generate synthetic training data |
| `finetype train` | Train CharCNN models (flat/tiered). `--seed N` for deterministic. Auto-snapshots. |
| `finetype taxonomy` | Print taxonomy summary (`--full --output json` for all fields) |
| `finetype schema <key\|file>` | Type-level JSON Schema (by key/glob) or table-level (by file path, `--stats`, `--stdout`) |
| `finetype validate <file> <schema>` | Schema-driven quality gate → `.valid.csv`, `.invalid.csv`, `.errors.jsonl` (`--summary-only`) |
| `finetype load <file>` | Profile → runnable DuckDB CTAS (`--table-name`, `--limit N`, `--no-normalize-names`, `--enum-threshold N`) |
| `finetype mcp` | Start MCP server over stdio (8 tools: profile, infer, ddl, taxonomy, schema, validate, generate) |

### Training infrastructure

**Crate:** `finetype-train` — pure Rust ML training on Candle. Metal auto-detected on macOS.

**`TrainingRenderer` trait** (`crates/finetype-train/src/tui.rs`): Display-only interface for training progress. Two implementations:
- **`TuiRenderer`** — ratatui alternate-screen dashboard (live loss/accuracy charts, epoch table, progress bar). Feature-gated behind `tui` (default on). No `enable_raw_mode()` — safe for unattended overnight runs.
- **`LogRenderer`** — `tracing::info!` fallback. Used when TUI init fails or `--no-tui` is passed.

**`results.json`** is the canonical source of training metrics — written by the training loop to the model output directory (e.g., `models/sherlock-v7/results.json`). Contains a JSON array of `EpochMetrics`:
```json
[{"epoch": 0, "train_loss": 1.23, "val_loss": 1.10, "train_accuracy": 0.45, "val_accuracy": 0.52, "learning_rate": 0.0001, "epoch_time_secs": 95.2}, ...]
```
**Do not parse log files for training metrics.** Read `results.json` directly — overnight scripts, comparison queries, and post-hoc analysis should all use this structured output.

**`TrainingSummary`** is returned by `train_multi_branch()`: `best_epoch`, `best_val_accuracy`, `total_epochs`, `total_time_secs`, `epoch_metrics: Vec<EpochMetrics>`.

### Evaluation infrastructure

**Profile eval** (`eval/profile_eval.sh`) — Multi-branch+Sharpen with sherlock-v13: **212/227 (93.4% label, 93.8% domain)**. 35 datasets, 227 columns, 240-type taxonomy. Run on Mac with `FINETYPE_MODEL_DIR=models/sherlock-v13`.
**Actionability eval** — 96.7% transform success rate (494k/511k values). Below old baseline (99.9%) due to expanded eval set exposing format_string gaps.
**External benchmarks:** GitTables 1M (47.1% label), SOTAB CTA (43.6% label) — format-detectable subset only.
**Dashboard:** `make eval-report` generates `eval/eval_output/report.md`.

To add regression datasets: create CSV in `/home/hugh/datasets/`, add to `eval/datasets/manifest.csv` + `eval/schema_mapping.yaml`, run `make eval-mapping` → `make eval-report`.

## Sprint Goal

**Publish quality + close format gaps (m-17).** Three workstreams:
1. **Audit 15 remaining misclassifications** — discovery to determine if v14 retrain is warranted or if the remaining errors need taxonomy/rule changes. Hierarchical subtypes (data_uri→url, email_display→email, phone_e164→phone_number), country↔country_code, user_agent→jwt, decimal_number confusion.
2. **Fix actionability format_string gaps** — iso_8601, iso, dmy_short_dot have empty format_strings. Taxonomy definition fixes, not model changes.
3. **PII flag in JSON Schema** — card 0012. Add `x-finetype-pii` to schema output based on taxonomy type. No model changes.

Previous: m-16 COMPLETE — shipped sherlock-v13 (212/227, 93.4%) + v0.6.16 release.

## Decision Register

46 architectural decisions in `decisions/` (MADR format). Key decisions — do not revisit without good reason.

Browse: `ls decisions/` or use Ctrl+B (fzf + glow preview).

Covers: inference pipeline, model architecture, embeddings & hints, rules & disambiguation, taxonomy, validation, schema & validate command (0031–0033), training, evaluation methodology, and distribution.

## Build & Test

```bash
make setup              # Install git hooks (first time)
cargo build             # Build core, model, cli
cargo test              # Run test suite
cargo run -- check      # Validate taxonomy/generator alignment
make ci                 # fmt + clippy + test + check
cargo build -p finetype_duckdb --release  # DuckDB extension
make eval-report        # Profile eval + actionability + dashboard

# Golden integration tests (profile, load, taxonomy, schema — ~2min)
cargo test -p finetype-cli --test cli_golden -- --ignored

# Training workflow scripts (Metal auto-detected on macOS)
./scripts/train.sh --samples 1000 --size small --epochs 5   # Quick training run
./scripts/train.sh --samples 5000 --size large --epochs 15  # Large model (M1 Metal)
./scripts/eval.sh --model models/char-cnn-v13               # Evaluate a trained model
./scripts/package.sh models/char-cnn-v13                     # Package for distribution
```

## Key File Reference

| What | Where |
|---|---|
| Taxonomy definitions | `labels/definitions_*.yaml` (7 domain files) |
| Column disambiguation + pipeline | `crates/finetype-model/src/column.rs` |
| Sense classifier | `crates/finetype-model/src/sense.rs` |
| Header hints (semantic) | `crates/finetype-model/src/semantic.rs` |
| Sibling-context attention | `crates/finetype-model/src/sibling_context.rs` |
| Table validator (core engine) | `crates/finetype-core/src/table_validator.rs` |
| CLI entry point | `crates/finetype-cli/src/main.rs` |
| MCP server + tools | `crates/finetype-mcp/src/` |
| DuckDB extension | `crates/finetype-duckdb/src/` |
| Training crate | `crates/finetype-train/src/` |
| Training TUI + renderer trait | `crates/finetype-train/src/tui.rs` |
| Multi-branch training loop | `crates/finetype-train/src/multi_branch.rs` |
| Training metrics (per model) | `models/<name>/results.json` |
| Eval binaries | `crates/finetype-eval/src/bin/` |
| Golden integration tests | `crates/finetype-cli/tests/cli_golden.rs` |
| Eval config + schema mapping | `eval/config.env`, `eval/schema_mapping.yaml` |
| CI workflow | `.github/workflows/ci.yml` |
| Overnight retraining script | `scripts/overnight_v11_retraining.sh` |
| Training/eval/package scripts | `scripts/train.sh`, `scripts/eval.sh`, `scripts/package.sh` |

## Workflow (orbit)

This project uses the orbit workflow: Card → Interview → Spec → Review → Ship.

- `/orb:card` — capture a feature need with expected behaviours
- `/orb:discovery` — explore a vague idea through Socratic Q&A
- `/orb:design` — refine a feature card into technical decisions
- `/orb:spec` — crystallise interview into a structured specification
- `/orb:review-spec` — stress-test the spec before implementation
- `/orb:review-pr` — verify the PR against the spec's acceptance criteria

Artifacts live in `cards/`, `specs/`, and `decisions/`.
