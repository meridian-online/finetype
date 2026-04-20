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

**Version:** 0.6.17
**Taxonomy:** 240 definitions across 7 domains (container: 11, datetime: 84, finance: 28, geography: 25, identity: 33, representation: 33, technology: 26) — all generators pass, 100% alignment
**Default model:** Multi-branch→Sharpen pipeline with sherlock-v16 (5-branch: char+embed+stats+header+validation). Single forward pass per column replaces ~100 CharCNN value-level inferences. Profile eval: **235/242 (97.1% label, 96.3% domain)** on 35 datasets (242 columns after ground truth corrections). Legacy Sense→Sharpen path remains in code but multi-branch is the CLI/MCP/DuckDB default.
**Features:** 36-dim deterministic feature extractor, column-level aggregation (mean, variance, min, max), 6 feature-based disambiguation rules (F1–F6), 23 value-based disambiguation rules (R1–R31, some gaps), Model2Vec semantic header hints, hardcoded header hints with domain-dependent thresholds (same-domain 0.95, cross-domain 0.85), datetime specificity guard (prevents iso_8601 catch-all from overriding specific datetime predictions).
**Codebase:** ~20k lines of Rust across 9 crates (including finetype-train for pure Rust ML training, finetype-mcp for MCP server). Zero Python dependencies (build + runtime).
**CI status:** All checks pass (fmt, clippy, test, taxonomy check). 413 model tests, zero warnings.
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged), MCP server (`finetype mcp`)

### Recent work

- **v0.6.17 release (SHIPPED 2026-04-20)** — Promoted sherlock-v16 as `models/default`, published to HuggingFace, bumped version 0.6.16 → 0.6.17, tagged `v0.6.17`. 5-platform binaries live (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap auto-bumped. Known narrow regression: single-value email in column mode — tracked in `specs/2026-04-20-v16-n1-email-regression/`. Release: https://github.com/meridian-online/finetype/releases/tag/v0.6.17. Handover: `specs/2026-04-20-v16-release/handover.md`.
- **v16 data audit + retrain (m-18, COMPLETE)** — Corrected 17 eval ground truth labels, added 15 new GT columns (242 total, was 227). Fixed `label_remap.json` broken chains (description/title/sentence/paragraph → plain_text). Split `_DROP_ALL_TYPES` into `_DROP_DISTILLED_TYPES` (7 types, mislabeled distilled rows) and `_DROP_SYNTHETIC_TYPES` (empty — generators retained). Added 4 value-pattern filters for distilled data quality. Fixed eval env var (`FINETYPE_MODEL_DIR` → `FINETYPE_MODEL`) and BSD `ln -sf` gotcha in promotion. Final 3-seed sweep (seeds 42/43/44 × 100 epochs): seed 43 won at 235/242 (97.1%). Net +2 over v14: 3 fixes (phone/method/hostname), 1 regression (fiscal_year→year, domain still correct). Decision: 0049. Spec: `specs/2026-04-18-v16-data-audit-retrain/`.
- **v15 Option C — selective hint narrowing** — Narrowed 7 harmful `header_hint()` keyword matches, added R25/R27 value-based rules. Profile eval: **215→218/227 (+3 net)**. Decision 0048 (value-based rules only). Spec: `specs/2026-04-18-v15-value-rules/`.
- **v14 model trained and promoted** — 50 epochs, 127 min on Metal, best val_acc 91.2%. Symlink `models/default → sherlock-v14`.
- **v13 retrain — data quality + architecture** (m-16) — Distilled data decontamination, validation branch resize. Profile eval: **201→212/227 (+11)**. Specs: `specs/2026-04-16-v13-retrain/`.

### What's next

- **CI hygiene — decouple from `models/default`** — `.github/scripts/download-model.sh` reads the live symlink at CI time. This forces release + model publish to interleave awkwardly (as seen in the v0.6.17 release). Option A (env-var pin) chosen; spec drafted with 6 ACs. Spec: `specs/2026-04-20-ci-decouple-default-symlink/spec.yaml`. ~1–2 hours.
- **N=1 email regression in v16** — single-value email in `--mode column` returns `plain_text`; v14 returned `email`. At N≥5 v16 classifies correctly and IPv4/URL don't regress. Narrow, edge-case, but worth understanding. Interview: `specs/2026-04-20-v16-n1-email-regression/interview.md`.
- **v16's 7 remaining errors on corrected eval** — user_agent (×2, still misclassified as jwt/docker_ref), geojson→plain_text, phone→ssn (people_directory), id→username (earthquakes_2024), fiscal_year→year (datetime_coverage, regression vs v14), locale→categorical (multilingual). Candidates for a follow-up data-quality card; none resolvable by simple disambiguation rules (blocked by decision 0048).
- **Fix distilled data for swift_bic / http_method / cpt / loinc / excel_format / ssn / user_agent** (follow-up to decision 0049) — relabel the ~45 mislabeled distilled rows so the 7 types have real distilled coverage on top of synthetic. Separate card.
- **2 actionability below-target** — multilingual/date (33.3%), tech_systems/version (93.8%). Both misclassification-driven; may resolve if the 7 v16 errors are addressed.

### Architectural direction (settled — do not re-ask)

- **Multi-branch as Sense replacement** (decision 0041): The multi-branch model (sherlock-v4-sibling) is the default classifier. It replaces both Sense and CharCNN — single forward pass per column. The Sharpen layer (feature_sharpen, value_sharpen, header hints) post-processes multi-branch output. Rules are progressively retired as the model improves.
- **Remove regex header hints** (decision 0042): Regex-based `header_hint()` and hardcoded header rules are deprecated in favour of learned approaches — multi-branch header branch (Model2Vec), sibling-context attention, and Model2Vec semantic matching. No more regex rabbit holes. v15 Option C furthered this by narrowing 7 harmful keyword matches.
- **Value-based rules only** (decision 0048): New disambiguation rules must check actual column values, not header metadata. Header-dependent disambiguation waits for model improvements.
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
   - `value_sharpen()`: R1–R31 rules on label + values + confidence
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

Labels: `domain.category.type` (e.g., `identity.person.email`). 7 domains: container (11), datetime (84), finance (28), geography (25), identity (33), representation (33), technology (26).

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

**`results.json`** is the canonical source of training metrics — written **incrementally** (atomic temp+rename after each epoch) to the model output directory (e.g., `models/sherlock-v16/results.json`). Contains a JSON array of `EpochMetrics`:
```json
[{"epoch": 0, "train_loss": 1.23, "val_loss": 1.10, "train_accuracy": 0.45, "val_accuracy": 0.52, "learning_rate": 0.0001, "epoch_time_secs": 95.2}, ...]
```
**`epochs.jsonl`** — one compact JSON line per epoch, appended during training. Survives `tee` capture (unlike TUI escape codes). Use this for `tail -f` monitoring:
```bash
tail -f models/sherlock-v16/epochs.jsonl
# {"epoch":1,"train_loss":5.4321,"val_loss":5.1234,"train_acc":0.0512,"val_acc":0.0634,"lr":1.00e-4,"time":152.3}
```
**Do not parse log files for training metrics.** Read `results.json` or `epochs.jsonl` directly — overnight scripts, comparison queries, and post-hoc analysis should all use this structured output.

**`TrainingSummary`** is returned by `train_multi_branch()`: `best_epoch`, `best_val_accuracy`, `total_epochs`, `total_time_secs`, `epoch_metrics: Vec<EpochMetrics>`.

### Evaluation infrastructure

**Profile eval** (`eval/profile_eval.sh`) — Multi-branch+Sharpen with sherlock-v16: **235/242 (97.1% label, 96.3% domain)**. 35 datasets, 242 columns (corrected ground truth, 17 labels fixed, 15 new columns added). 240-type taxonomy. Timestamp interchangeability tightened to format-compatible families (ISO, SQL, RFC 2822, MDY, DMY, YMD). **Note:** `FINETYPE_MODEL_DIR` env var is for the DuckDB extension only. The CLI uses `--model` flag (default: `models/default`). The eval script respects `FINETYPE_MODEL` env var (passed as `--model`).
**Actionability eval** — 578903/578949 (100%) transform success rate. 2 below-target columns remaining (multilingual/date, tech_systems/version).
**External benchmarks:** GitTables 1M (47.1% label), SOTAB CTA (43.6% label) — format-detectable subset only.
**Dashboard:** `make eval-report` generates `eval/eval_output/report.md`.

To add regression datasets: create CSV in `/Users/hugh/datasets/finetype/`, add to `eval/datasets/manifest.csv` + `eval/schema_mapping.yaml`, run `make eval-mapping` → `make eval-report`.

## Sprint Goal

**m-18 COMPLETE — v16 retrain shipped.** Corrected eval ground truth
(+17 labels fixed, +15 new columns → 242 total), split `_DROP_ALL_TYPES`
to preserve synthetic data for the 7 types with bad distilled data, fixed
the eval env-var bug (`FINETYPE_MODEL_DIR` → `FINETYPE_MODEL`) and the
BSD `ln -sf` promotion bug. Seed 43 promoted at **235/242 (97.1%)**, net
+2 over v14. Decision: 0049. Spec: `specs/2026-04-18-v16-data-audit-retrain/`.

**Next sprint (m-19 — TBD):** candidates include v0.6.17 release, distilled-
data relabelling for the 7 still-error-prone types, or the deferred PII
detection card (`specs/2026-04-18-pii-detection/`).

Previous: m-17 COMPLETE — PII detection (card 0012), actionability audit P1–P5, v15 Option C (233/242 on corrected eval, 96.3%).

## Decision Register

46 architectural decisions in `decisions/` (MADR format). Key decisions — do not revisit without good reason.

Browse: `ls decisions/` or use Ctrl+B (fzf + glow preview).

Covers: inference pipeline, model architecture, embeddings & hints, rules & disambiguation, taxonomy, validation, schema & validate command (0031–0033), training, evaluation methodology, and distribution.

## Release & Model Promotion

### Model-name env vars

Three env vars exist — each is read by exactly one consumer. Do not conflate.

```
| Env var              | Consumer                           | Purpose                                     |
|----------------------|------------------------------------|---------------------------------------------|
| FINETYPE_CI_MODEL    | .github/scripts/download-model.sh  | CI's authoritative model name for fetches   |
| FINETYPE_MODEL       | CLI, eval scripts                  | Path passed to `finetype --model`           |
| FINETYPE_MODEL_DIR   | DuckDB extension                   | Local path override (bypasses HF download)  |
```

CLI/MCP/DuckDB/eval code does NOT read `FINETYPE_CI_MODEL`. The runtime
default remains `models/default` for every non-CI consumer.

### Promotion flow (new model → release)

After the v0.6.17 release we decoupled CI from the `models/default` symlink
(see `specs/2026-04-20-ci-decouple-default-symlink/`). The new 3-step flow:

1. **Publish to HuggingFace** — upload the trained model directory to
   `meridian-online/finetype-model` on HF.
2. **Bump `FINETYPE_CI_MODEL`** in `.github/workflows/ci.yml` and
   `.github/workflows/release.yml` (workflow-level `env:` blocks).
3. **Flip `models/default`** — `ln -sfn <new-model> models/default`.

Steps 2 and 3 may ship in the same PR. Step 1 must precede step 2 (or step 2
can be deferred if the promotion is purely a runtime change).

A non-blocking drift check (`.github/scripts/check-ci-model-drift.sh`) warns
in CI when `FINETYPE_CI_MODEL` and `models/default` disagree — legitimate
during promotion PRs, but visible so divergence isn't silent for weeks.

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
| Training metrics (per model) | `models/<name>/results.json`, `models/<name>/epochs.jsonl` |
| Eval binaries | `crates/finetype-eval/src/bin/` |
| Golden integration tests | `crates/finetype-cli/tests/cli_golden.rs` |
| Eval config + schema mapping | `eval/config.env`, `eval/schema_mapping.yaml` |
| CI workflow | `.github/workflows/ci.yml` |
| v16 sweep script | `scripts/sweep_v16.sh` |
| v16 overnight retraining | `scripts/overnight_v16_retraining.sh` |
| Data preparation (multi-branch) | `scripts/prepare_multibranch_data.py` |
| Label remap (distilled→canonical) | `data/label_remap.json` |
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
