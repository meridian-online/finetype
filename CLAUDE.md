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

- **`finetype validate` — DuckDB-native reject pipeline (SHIPPED 2026-04-24, PR #46)** — Replaces the CSV-sidecar validate flow with a single `.db` output: user's table (valid rows only) + `finetype_reject_errors` sidecar mirroring DuckDB's `reject_errors` base (9 cols) with FineType extensions (`type_confidence`, `expected_type`, `constraint_failed`, `constraint_value`). Single validation engine: `finetype-core::table_validator::validate_table` is the sole pass/fail source; CLI shells out to `duckdb` binary with a TEMPORARY staging table (RAII-equivalent cleanup on success AND failure). Scalar `finetype_validate(value, schema_json)` preserved unchanged. Exit codes: 0 no rejects / 1 rejects / 2 error; `--lenient` forces 0; `--append` increments scan_id. ac-04 spike ratified Scenario A (duckdb-rs 1.4.4 `BindInfo` has no `Connection` → no new DuckDB table function); `spike.rs` retained as living evidence. 14/14 ACs, 9/9 constraints, 15 `vrp_*` tests across finetype-core (7) + finetype-cli (8). Decision: 0064 (refines 0031/0032). Spec: `orbit/specs/2026-04-22-duckdb-extension-ergonomics/`.
- **v18 retrain (HELD 2026-04-22, not promoted, branch only)** — Full-auto sweep on v3 corpus (decision 0060) with fixed data-seed discipline (decision 0061). 3 seeds × 100 epochs completed in 440 min. All 3 seeds AUTO_ACCEPT (val_acc ≥0.912): 42=0.9134 / 43=0.9130 / 44=0.9133. Winner seed 42 at **297/352 (84.4% label, 92.3% domain)** — ties v16 exactly. Per-column diff: 8 fixes / 8 regressions / 47 persistent (44 same-prediction, 3 churn). Per-domain max regression: datetime +3 (at gate limit). Decision 0062: **HELD** — net-zero delta matches v17 precedent. v16 remains shipped. Follow-ups: amount-variant / container-type / datetime-subtype generator cards for v19. Spec: `orbit/specs/2026-04-21-v18-retrain/`.
- **Sharpen demotion guard (SHIPPED 2026-04-21, PR #44)** — Validator-confirmed demotion guard at top of `disambiguate_categorical` (column.rs:3902). Added `Validation::is_precise()` predicate + 240-row taxonomy audit (224 precise / 16 imprecise). Full eval gate: `regressions: 0, improvements: 1, neutral: 5`. Decision 0059 accepted. Honest outcome: `excel_format` column under 110-col sibling-context attention yields `representation.text.word`, not `excel_format` — the guard preserves raw-model top-1; it does not promote. `http_method` behaves the same way under sibling-context; in per-column paths (`load`, single-column profile) the guard successfully rescues `technology.internet.http_method`. Spec: `orbit/specs/2026-04-21-sharpen-demotion-guard/`.
- **Eval-corpus expansion Phase A+B (SHIPPED 2026-04-21, PR #42)** — Three deliverables, all shipping: (1) realism standard + programmatic pre-screen at `scripts/prescreen_eval.py` against floors in MADR 0055; (2) coverage floor at `scripts/eval_coverage_check.py` — 240/240 types covered via `eval/datasets/csv/coverage_closure_phase_ab.csv` (110 cols × 6 rows); (3) train/eval leakage firewall — two-layer (source-level `eval/datasets/sources.yaml`, 35 sources, role=eval + row-hash SHA256 filter at `scripts/prepare_multibranch_data.py` over 237,860 distinct rows). Manifest schema migrated 4→7 columns (`source_url`, `licence`, `fetched_date`). Decisions 0055/0056/0057 accepted. v16 diagnostic re-score on expanded corpus: **297/352 (84.4% label, 91.8% domain)** — expected drop as newly-covered types surface v16's weak spots. Spec: `orbit/specs/2026-04-21-eval-expansion/`.
- **v17 re-eval on expanded corpus (2026-04-21, MADR 0058)** — Decision 0054 held v17 pending expanded-eval measurement. Re-scored v17-seed-44 against the 448-row manifest: v17 **does not** outperform v16 (295/352 vs 297/352 label; +3 domain; 6 stable-hit / 4 stable-miss / 0 fix / 0 regression on the 10 relabel-target rows). Signal-to-noise argument: 45 relabeled rows / 7000+ training rows per type = 0.6%, below the noise floor. **v17 not promoted; `models/default` stays at sherlock-v16.** Surfaced pipeline gap: `http_method` and `excel_format` predict generic `representation.discrete.categorical` when a validator-authoritative promotion should lift to the named type — concrete, spec-able, no retrain needed (addressed by PR #44 / decision 0059). Spec: `orbit/specs/2026-04-21-v17-re-eval/`. Decision: 0058.
- **v17 distilled-data relabel (HELD, not promoted, branch only)** — Decision 0054. Distilled data fix for 7 short-string types (swift_bic, http_method, cpt, loinc, excel_format, ssn, user_agent) via v4 loaders + generator widening + http_method ENUM-only. 3-seed sweep completed cleanly (seeds 42/43/44, 100 epochs, patience-15 early-stop). Winner seed 44 at 235/242 (identical eval to v16), but per-column diff showed 3 fixes / 3 non-target regressions / 2 persistent user_agent failures — net zero. Decision: **v16 remains shipped**, v4 artefacts stay on branch `distilled-data-relabel-7-types-v17` for future reuse. Decisions 0050/0051/0052/0053. Spec: `orbit/specs/2026-04-20-distilled-data-relabel-7-types/`.
- **CI hygiene — decouple download-model.sh from `models/default` (SHIPPED 2026-04-20, PR #39)** — Introduced `FINETYPE_CI_MODEL` workflow-level env var as CI's authoritative model name. `models/default` remains the runtime default for CLI/MCP/DuckDB/eval. New `download-model-test` and `drift-check` jobs in CI. Future promotion PRs no longer require the "HuggingFace first, then flip symlink" dance. Spec: `orbit/specs/2026-04-20-ci-decouple-default-symlink/` (interview + spec.yaml v1.1 + review-spec + review-pr + progress).
- **v0.6.17 release (SHIPPED 2026-04-20)** — Promoted sherlock-v16 as `models/default`, published to HuggingFace, bumped version 0.6.16 → 0.6.17, tagged `v0.6.17`. 5-platform binaries live (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap auto-bumped. Known narrow regression: single-value email in column mode — tracked in `orbit/specs/2026-04-20-v16-n1-email-regression/`. Release: https://github.com/meridian-online/finetype/releases/tag/v0.6.17. Handover: `orbit/specs/2026-04-20-v16-release/handover.md`.
- **v16 data audit + retrain (m-18, COMPLETE)** — Corrected 17 eval ground truth labels, added 15 new GT columns (242 total, was 227). Fixed `label_remap.json` broken chains (description/title/sentence/paragraph → plain_text). Split `_DROP_ALL_TYPES` into `_DROP_DISTILLED_TYPES` (7 types, mislabeled distilled rows) and `_DROP_SYNTHETIC_TYPES` (empty — generators retained). Added 4 value-pattern filters for distilled data quality. Fixed eval env var (`FINETYPE_MODEL_DIR` → `FINETYPE_MODEL`) and BSD `ln -sf` gotcha in promotion. Final 3-seed sweep (seeds 42/43/44 × 100 epochs): seed 43 won at 235/242 (97.1%). Net +2 over v14: 3 fixes (phone/method/hostname), 1 regression (fiscal_year→year, domain still correct). Decision: 0049. Spec: `orbit/specs/2026-04-18-v16-data-audit-retrain/`.
- **v15 Option C — selective hint narrowing** — Narrowed 7 harmful `header_hint()` keyword matches, added R25/R27 value-based rules. Profile eval: **215→218/227 (+3 net)**. Decision 0048 (value-based rules only). Spec: `orbit/specs/2026-04-18-v15-value-rules/`.
- **v14 model trained and promoted** — 50 epochs, 127 min on Metal, best val_acc 91.2%. Symlink `models/default → sherlock-v14`.
- **v13 retrain — data quality + architecture** (m-16) — Distilled data decontamination, validation branch resize. Profile eval: **201→212/227 (+11)**. Specs: `orbit/specs/2026-04-16-v13-retrain/`.

### What's next

- **v19 retrain — UNBLOCKED by v18 HELD (decision 0062).** v18 proved retraining alone is not the lever — 47/55 v16 failures persist unchanged. The v18 diff names three follow-up generator cards: (1) amount-variant generators (11 persistent misses collapsing to plain `amount`), (2) container-type generators (8 collapsing to `categorical`), (3) datetime-subtype generators (6 collapsing to nearest-but-wrong timestamp). v19 sprint-open after a /orb:discovery on the first cluster. v4-UA-adoption card (from decision 0060 follow-up) also carries forward.
- **N=1 email regression in v16** — single-value email in `--mode column` returns `plain_text`; v14 returned `email`. At N≥5 v16 classifies correctly and IPv4/URL don't regress. Narrow, edge-case, but worth understanding. Interview: `orbit/specs/2026-04-20-v16-n1-email-regression/interview.md`. Next step: `/orb:discovery` to reproduce + identify cause, then `/orb:spec`.
- **2 actionability below-target** — multilingual/date (33.3%), tech_systems/version (93.8%). Both misclassification-driven; may resolve after v18.

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
| `finetype validate <file> <schema> --db <out.db> --table <name>` | Schema-driven quality gate → DuckDB `.db` file with user table (valid rows) + `finetype_reject_errors` sidecar (13-col DuckDB `reject_errors` shape + FineType extensions `type_confidence`, `expected_type`, `constraint_failed`, `constraint_value`). Flags: `--append` (reuse db, scan_id++), `--lenient` (force exit 0). Exit codes: 0 no rejects / 1 rejects / 2 error. Requires `duckdb` on PATH. See MADR 0064. |
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

**Profile eval** (`eval/profile_eval.sh`) — Multi-branch+Sharpen with sherlock-v16: **235/242 (97.1% label, 96.3% domain)** on the pre-expansion 242-column eval. Post-eval-expansion (m-19): **297/352 (84.4% label, 91.8% domain)** diagnostic re-score on the expanded 448-row manifest with 110 newly-covered types surfaced (see `orbit/specs/2026-04-21-eval-expansion/`). Manifest schema: 7 columns (`dataset, file_path, column_name, gt_label, source_url, licence, fetched_date`). 240-type taxonomy. Timestamp interchangeability tightened to format-compatible families (ISO, SQL, RFC 2822, MDY, DMY, YMD). **Note:** `FINETYPE_MODEL_DIR` env var is for the DuckDB extension only. The CLI uses `--model` flag (default: `models/default`). The eval script respects `FINETYPE_MODEL` env var (passed as `--model`).
**Actionability eval** — 578903/578949 (100%) transform success rate. 2 below-target columns remaining (multilingual/date, tech_systems/version).
**External benchmarks:** GitTables 1M (47.1% label), SOTAB CTA (43.6% label) — format-detectable subset only.
**Dashboard:** `make eval-report` generates `eval/eval_output/report.md`.

To add regression datasets: create CSV in `/Users/hugh/datasets/finetype/`, add to `eval/datasets/manifest.csv` + `eval/schema_mapping.yaml`, run `make eval-mapping` → `make eval-report`.

## Sprint Goal

**m-19 IN FLIGHT — eval-corpus expansion (Phase A+B).** Closing the
realism / coverage / leakage gaps surfaced by the v17 hold (decision
0054). Three deliverables, all shipping before any v18 sweep may start:

1. **Realism standard + programmatic pre-screen + human-reviewed triage.**
   `scripts/prescreen_eval.py` computes 6 metrics (null_rate,
   unique_ratio, whitespace_ratio, format_variance, shannon_entropy,
   top_1_skew) against the pinned floors in MADR 0055; triage.md is a
   draft worklist (338 keep / 0 augment / 0 replace on existing rows)
   awaiting Hugh's human review. Restricted-registry carve-out for 6
   types (identity.medical.cpt/loinc, identity.government.ssn/ein,
   finance.banking.swift_bic, finance.payment.credit_card_number) under
   `provenance_status = synthetic-necessary`.
2. **Coverage floor — every taxonomy type has ≥1 eval column.**
   `scripts/eval_coverage_check.py` is the machine gate.
   `eval/datasets/csv/coverage_closure_phase_ab.csv` (110 columns × 6
   rows) closes the zero-coverage gap; schema_mapping.yaml extended by
   110 identity mappings. Final: 240/240 types covered, exit 0.
3. **Train/eval leakage firewall — two-layer defence.** (a) Source-level
   role manifest at `eval/datasets/sources.yaml` (35 sources, role=eval).
   (b) Row-hash SHA256 filter at `scripts/prepare_multibranch_data.py`
   (active-by-default) over normalised (header, sample-values) via shared
   normaliser at `scripts/eval_leakage/__init__.py`. Hash table at
   `eval/row_hashes.tsv` regenerated against the expanded 448-row
   manifest (237,860 distinct rows across 441 columns).

**Manifest schema migrated 4→7 columns** (`source_url`, `licence`,
`fetched_date`). `eval/profile_eval.sh` patched to read 7 fields;
Rust `csv::Reader` consumers tolerant via `.get(index)` semantics.
**v18 retrain block** enforced in `scripts/sweep_v17.sh` header comment
+ sprint policy: no v18 model sweep may start until Phase A+B ships.

**Decisions:** 0055 (realism dimensions), 0056 (leakage prevention),
0057 (coverage floor) — all drafted `proposed`, move to `accepted`
after verifying ACs ship. Card: `orbit/cards/0002-semantic-type-detection.yaml`.
Spec: `orbit/specs/2026-04-21-eval-expansion/spec.yaml`.

**v16 diagnostic re-score on expanded eval:** 297/352 (84.4% label,
91.8% domain) — expected drop from 235/242 (97.1%) as newly-covered
types surface v16's weak spots. Diagnostic only, not a promotion
baseline. Phase C (edge-case second-column coverage per type) is
explicitly out of scope.

Previous: m-18 COMPLETE — v16 retrain shipped at 235/242 (97.1%) on
corrected eval. Spec: `orbit/specs/2026-04-18-v16-data-audit-retrain/`.
m-17 COMPLETE — PII detection (card 0012), actionability audit P1–P5,
v15 Option C (233/242 on corrected eval).

## Decision Register

46 architectural decisions in `orbit/decisions/` (MADR format). Key decisions — do not revisit without good reason.

Browse: `ls orbit/decisions/` or use Ctrl+B (fzf + glow preview).

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
(see `orbit/specs/2026-04-20-ci-decouple-default-symlink/`). The new 3-step flow:

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
| Eval manifest (7-col, 448 rows) | `eval/datasets/manifest.csv` |
| Source role manifest | `eval/datasets/sources.yaml` |
| Row-hash leakage firewall | `eval/row_hashes.tsv`, `scripts/eval_leakage/__init__.py`, `scripts/compute_row_hashes.py` |
| Eval pre-screen | `scripts/prescreen_eval.py`, `eval/pre-screen_floors.yaml` |
| Coverage gate | `scripts/eval_coverage_check.py`, `eval/datasets/csv/coverage_closure_phase_ab.csv` |
| Licence allowlist | `eval/licence_allowlist.txt` |

## Workflow (orbit)

This project uses the orbit workflow: Card → Interview → Spec → Review → Ship.

- `/orb:card` — capture a feature need with expected behaviours
- `/orb:discovery` — explore a vague idea through Socratic Q&A
- `/orb:design` — refine a feature card into technical decisions
- `/orb:spec` — crystallise interview into a structured specification
- `/orb:review-spec` — stress-test the spec before implementation
- `/orb:review-pr` — verify the PR against the spec's acceptance criteria

Artefacts live in `orbit/cards/`, `orbit/specs/`, and `orbit/decisions/`.
