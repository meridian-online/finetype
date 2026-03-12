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
2. **Write programs that do one thing and do it well** — FineType infers types. It doesn't validate, transform, or visualise. Those are separate concerns for separate tools.
3. **Design for the future, for it will be here sooner than you think** — The type taxonomy, model architecture, and extension interfaces should accommodate new data types and formats without breaking existing behaviour.

### Precision Principle

Precision is what makes FineType valuable. Every validation pattern, locale rule, and disambiguation heuristic must meaningfully distinguish "is this type" from "is not this type."

- **Prefer precise locale-specific validation over permissive universal patterns.** If a type is `designation: locale_specific`, its real validation lives in `validation_by_locale`, not the universal `validation` block.
- **A validation that confirms 90% of random input is not a validation.**
- **Expanding locale coverage is the path to accuracy**, not relaxing heuristics.

## Current State

**Version:** 0.6.10
**Taxonomy:** 250 definitions across 7 domains (container: 12, datetime: 84, finance: 31, geography: 25, identity: 34, representation: 36, technology: 28) — all generators pass, 100% alignment
**Default model:** Sense→Sharpen pipeline (CLI) with char-cnn-v14-250 flat (250 classes, 10 epochs, 372k samples), tiered-v2 fallback via `--sharp-only`. Hierarchical head available: char-cnn-v15-250 (7→43→250 tree softmax, 84.2% type / 90.9% domain / 96.5% category training accuracy).
**Features:** 36-dim deterministic feature extractor (NNFT-248/266/270), column-level aggregation (mean, variance, min, max), 5 feature-based disambiguation rules (F1–F5). Financial header hints (NNFT-270).
**Codebase:** ~20k lines of Rust across 9 crates (including finetype-train for pure Rust ML training, finetype-mcp for MCP server). Zero Python dependencies (build + runtime).
**CI status:** All checks pass (fmt, clippy, test, taxonomy check)
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged), MCP server (`finetype mcp`)

### Recent work

- **Sibling-context attention** (NNFT-268, m-13) — 2-layer pre-norm transformer self-attention on 509 real-world CSVs. Enriches per-column headers with cross-column context before Sense classification. Profile: 170/174 (97.7% label, 98.9% domain). Entry point: `classify_columns_with_context()`.
- **Hierarchical classification head** (NNFT-267, m-13) — Tree softmax (7 domains → 43 categories → 250 types). char-cnn-v15-250: 84.2% type accuracy. Matches flat baseline on profile eval. `--hierarchical` flag.
- **Sherlock-style features** (NNFT-270, m-12) — FEATURE_DIM 36 with financial header hints. Rules F1–F5 for disambiguation.

### What's in progress

- **Golden test expansion** (NNFT-258) — Rust integration tests covering profile, load, taxonomy, schema commands. Both small fixtures and real CSV datasets. Structured field matching (label, domain, confidence range). Depends on NNFT-254 completion.
- **Eval baseline reconciliation** — Profile eval count shifted from 186 to 174 matchable predictions. Need to identify cause (manifest changes, schema mapping updates, or sibling-context side effect).
- **Remaining accuracy gaps** — 4 misclassifications at 170/174: 3× bare "name" ambiguity (sibling context shifts to geographic types), 1× docker_ref→hostname.

## Architecture

### Workspace layout

```
finetype/
  crates/
    finetype-core/     # Taxonomy, generators, validation, tokenizer
    finetype-model/    # CharCNN, tiered classifier, column disambiguation, training
    finetype-cli/      # CLI binary (infer, profile, generate, check, train, mcp)
    finetype-mcp/      # MCP server (rmcp v1.1.0, 6 tools, taxonomy resources)
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
finetype-model (depends on core — CharCNN, tiered inference, column mode)
    |
    +--- finetype-cli   (depends on core + model + mcp — CLI binary)
    +--- finetype-mcp   (depends on core + model — MCP server library)
    +--- finetype-duckdb (depends on core + model — DuckDB extension)

finetype-eval  (standalone — eval binaries, depends on csv/parquet/duckdb/arrow)
```

### Inference pipeline

**Value-level:** Single string → type label via `CharClassifier` (flat, 250 classes) or `TieredClassifier` (34 CharCNN models). Both implement `ValueClassifier` trait.

**Column-level (Sense→Sharpen, default):** Vector of strings + header → single column type:
1. Optional sibling-context attention enriches headers with cross-column context
2. Sample 100 values, encode header with Model2Vec, extract 36-dim deterministic features
3. Sense classify → broad category (temporal/numeric/geographic/entity/format/text)
4. CharCNN batch inference on all values → masked vote aggregation (filtered by Sense category)
5. Disambiguation: vote-based rules, feature-based rules (F1–F5), entity demotion
6. Header hints (hardcoded + Model2Vec semantic) with geography protection
7. Post-hoc locale detection via `validation_by_locale` patterns

Key implementation files: `column.rs` (disambiguation + pipeline), `sense.rs` (Sense classifier), `semantic.rs` (header hints), `sibling_context.rs` (attention). Legacy fallback path exists when Sense model is absent.

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
| `finetype_version()` | Version string |

Uses flat CharCNN with chunk-aware column classification (~2048-row chunks).

### MCP server

`finetype mcp` starts an MCP server over stdio transport (rmcp v1.1.0). AI agents launch it as a subprocess.

**Tools (6):**

| Tool | Purpose |
|---|---|
| `infer` | Classify values (single or column mode with header) |
| `profile` | Profile all columns in CSV file (path or inline data) |
| `ddl` | Generate CREATE TABLE DDL from file profiling |
| `taxonomy` | Search/filter type taxonomy by domain/category/query |
| `schema` | Export JSON Schema contract for type(s), supports globs |
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
| `finetype schema <key>` | Export JSON Schema (`--pretty`, glob patterns, `x-finetype-*` DDL fields) |
| `finetype load <file>` | Profile → runnable DuckDB CTAS (`--table-name`, `--limit N`, `--no-normalize-names`, `--enum-threshold N`) |
| `finetype mcp` | Start MCP server over stdio (6 tools: profile, infer, ddl, taxonomy, schema, generate) |

### Evaluation infrastructure

**Profile eval** (`eval/profile_eval.sh`) — 97.7% label (170/174), 98.9% domain on 30 datasets (293 manifest entries, 250-type taxonomy).
**Actionability eval** — 99.9% transform success rate (232k values, 283 columns, 120 types).
**External benchmarks:** GitTables 1M (47.1% label), SOTAB CTA (43.6% label) — format-detectable subset only.
**Dashboard:** `make eval-report` generates `eval/eval_output/report.md`.

To add regression datasets: create CSV in `/home/hugh/datasets/`, add to `eval/datasets/manifest.csv` + `eval/schema_mapping.yaml`, run `make eval-mapping` → `make eval-report`.

## Sprint Goal

**Architecture evolution (m-13):** Sibling-context attention trained and shipped (v0.6.10). Hierarchical head accuracy parity, golden test expansion.

**Remaining accuracy gaps:** 4 misclassifications at 170/174 — 3× bare "name" ambiguity (genuinely ambiguous, sibling context shifts these to geographic types), 1× docker_ref/hostname confusion.

## Decision Register

30 architectural decisions in `decisions/` (MADR format). Key decisions — do not revisit without good reason.

Browse: `ls decisions/` or use Ctrl+B (fzf + glow preview).

Covers: inference pipeline, model architecture, embeddings & hints, rules & disambiguation, taxonomy, validation, training, evaluation methodology, and distribution.

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
| CLI entry point | `crates/finetype-cli/src/main.rs` |
| MCP server + tools | `crates/finetype-mcp/src/` |
| DuckDB extension | `crates/finetype-duckdb/src/` |
| Training crate | `crates/finetype-train/src/` |
| Eval binaries | `crates/finetype-eval/src/bin/` |
| Golden integration tests | `crates/finetype-cli/tests/cli_golden.rs` |
| Eval config + schema mapping | `eval/config.env`, `eval/schema_mapping.yaml` |
| CI workflow | `.github/workflows/ci.yml` |
| Training/eval/package scripts | `scripts/train.sh`, `scripts/eval.sh`, `scripts/package.sh` |

## Workflow

**Seed-driven:** interview → decision → seed → implement via PR → evaluate. No backlog, no task tracking.
**Specs** live in `specs/`. **Decisions** live in `decisions/`. **Code changes** ship via PRs.

<!-- ooo:START -->
<!-- ooo:VERSION:0.14.0 -->
# Ouroboros — Specification-First AI Development

> Before telling AI what to build, define what should be built.
> As Socrates asked 2,500 years ago — "What do you truly know?"
> Ouroboros turns that question into an evolutionary AI workflow engine.

Most AI coding fails at the input, not the output. Ouroboros fixes this by
**exposing hidden assumptions before any code is written**.

1. **Socratic Clarity** — Question until ambiguity ≤ 0.2
2. **Ontological Precision** — Solve the root problem, not symptoms
3. **Evolutionary Loops** — Each evaluation cycle feeds back into better specs

```
Interview → Seed → Execute → Evaluate
    ↑                           ↓
    └─── Evolutionary Loop ─────┘
```

## ooo Commands

Each command loads its agent/MCP on-demand. Details in each skill file.

| Command | Loads |
|---------|-------|
| `ooo` | — |
| `ooo interview` | `ouroboros:socratic-interviewer` |
| `ooo seed` | `ouroboros:seed-architect` |
| `ooo run` | MCP required |
| `ooo evolve` | MCP: `evolve_step` |
| `ooo evaluate` | `ouroboros:evaluator` |
| `ooo unstuck` | `ouroboros:{persona}` |
| `ooo status` | MCP: `session_status` |
| `ooo setup` | — |
| `ooo help` | — |

## Agents

Loaded on-demand — not preloaded.

**Core**: socratic-interviewer, ontologist, seed-architect, evaluator,
wonder, reflect, advocate, contrarian, judge
**Support**: hacker, simplifier, researcher, architect
<!-- ooo:END -->
