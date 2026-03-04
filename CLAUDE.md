# FineType

FineType is a type inference engine that detects and classifies data types in tabular datasets. It's the core analytical engine of the Noon project.

## The Noon Pillars

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

**Version:** 0.5.2
**Taxonomy:** 163 definitions across 7 domains (container: 12, datetime: 45, finance: 16, geography: 15, identity: 19, representation: 32, technology: 24) — all generators pass, 100% alignment
**Default model:** Sense→Sharpen pipeline (CLI) with char-cnn-v11 flat (163 classes, 10 epochs), tiered-v2 fallback via `--sharp-only`
**Codebase:** ~20k lines of Rust across 8 crates (including finetype-train for pure Rust ML training). Zero Python dependencies (build + runtime).
**CI status:** All checks pass (fmt, clippy, test, taxonomy check)
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged)

### Recent milestones

- **Locale Foundation expansion** (NNFT-195–201) — Layer 1: Expanded validation to 50+ postal codes, 45+ phone numbers, 30+ month/day names. Layer 2: Expanded generators to match (65 postal locales, 46 phone locales, 32 CLDR date/time patterns). CharCNN-v11 retrained on expanded data (10 epochs, 88.3% training accuracy). Profile eval improved 110/116→112/116 (96.6%). Actionability at 95.4% (rfc_2822 misclassification — tracked in NNFT-194).
- **Taxonomy revision v0.5.2** (NNFT-192) — Removed `geography.address.street_number` (false positives on plain integers) and `identity.person.age` (indistinguishable from integer_number, 205 SOTAB false positives). Added `representation.identifier.numeric_code` (VARCHAR, preserves leading zeros for codes like ISO country numeric, NAICS, FIPS). Net: 164→163 types. CharCNN-v10 retrained. Actionability improved 96.0%→98.7%. Profile eval regressed 117/119→110/116 due to model retrain — follow-up needed.
- **Actionability improvements** (NNFT-191) — Actionability 92.7% → 96.0% (2910/3030 values). Added `format_string_alt` field to taxonomy YAML for ISO 8601 fractional seconds variant. Updated eval to try multiple format strings per type. Fixed network_logs.timestamp (0% → 100%).
- **Accuracy improvements** (NNFT-188) — Profile eval 108/119 → 117/119 (98.3% label, 99.2% domain). Six mechanisms: validation-based candidate elimination, Rule 19, header hint additions, hardcoded hint priority over Model2Vec, same-domain geo override, geography rescue from unmasked votes.
- **v0.5.1 model retrain** (NNFT-181) — All models retrained on clean 164-type taxonomy. CharCNN-v9 (1,000 samples/type), refreshed Model2Vec type embeddings, Sense + Entity classifiers.
- **Pure Rust training** (NNFT-185) — All Python training scripts replaced with Rust/Candle. `finetype-train` crate with 4 binaries. Zero Python dependencies.
- **Taxonomy v0.5.1** (NNFT-177/178/179/180) — Finance domain (banking, commerce), identifier category. 164 types across 7 domains.

### What's in progress

- **Post-retrain accuracy recovery** (NNFT-194) — Profile eval 112/116 (96.6%). 4 remaining misclassifications: utc_offset→excel_format, sports_events.venue→city, countries.name→full_name, world_cities.name→full_name. Actionability at 95.4% — rfc_2822_timestamp misclassified as iso_8601 (80 values). If fixed, actionability would be 98.0%.

## Architecture

### Workspace layout

```
finetype/
  crates/
    finetype-core/     # Taxonomy, generators, validation, tokenizer
    finetype-model/    # CharCNN, tiered classifier, column disambiguation, training
    finetype-cli/      # CLI binary (infer, profile, generate, check, train)
    finetype-duckdb/   # DuckDB loadable extension (scalar functions)
    finetype-eval/     # Evaluation binaries (report, actionability, GitTables, SOTAB)
    finetype-candle-spike/  # ML training feasibility spike (Candle 0.8)
    finetype-train/    # Pure Rust ML training (Sense, Entity, data pipeline)
    finetype-build-tools/  # Build utilities (DuckDB extension metadata)
  labels/              # Taxonomy YAML definitions (6 domain files)
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
    +--- finetype-cli   (depends on core + model — CLI binary)
    +--- finetype-duckdb (depends on core + model — DuckDB extension)

finetype-eval  (standalone — eval binaries, depends on csv/parquet/duckdb/arrow)
```

### Inference pipeline

**Value-level:** Single string → type label via `CharClassifier` (flat, 163 classes) or `TieredClassifier` (46 CharCNN models in T0→T1→T2 graph). Both implement `ValueClassifier` trait.

**Column-level (Sense→Sharpen, default):** Vector of strings + header → single column type:
1. Sample 100 values, encode header + first 50 with Model2Vec
2. Sense classify → broad category (temporal/numeric/geographic/entity/format/text) + entity subtype
3. Run flat CharCNN batch on all 100 values, remap collapsed labels
4. **Masked vote aggregation:** filter to category-eligible labels via `LabelCategoryMap`. Safety valve: falls back to unmasked when all votes filtered OR when Sense confidence <0.75 and masking removes >40% of votes
5. Apply disambiguation rules (same rules, votes already scoped). Coordinate disambiguation requires competitive vote share (prevents false-positive on decimal columns)
6. Entity demotion: non-person Sense subtype + full_name → entity_name (replaces Rule 18 + EntityClassifier)
7. **Header hints** (Model2Vec semantic + hardcoded): override generic/low-confidence predictions. Geography protection for person-name hints. Measurement disambiguation for age/height/weight
8. Post-hoc locale detection (unchanged)

**Column-level (legacy, when Sense absent):** Vector of strings → single column type:
1. Run value-level inference on each value
2. Remap collapsed type labels via `remap_collapsed_label()` (8 types redirected, NNFT-162)
3. Aggregate via majority vote
4. Apply disambiguation rules in order:
   - **Rule 14 — Duration override:** SEDOL + ISO 8601 P-prefix ≥50% → duration. Runs before attractor demotion.
   - **Rule 15 — Attractor demotion:** Three signals (validation failure >50%, confidence <0.85, cardinality 1-20). Locale-confirmed predictions skip Signals 2-3. Demoted → generic for header hints.
   - **Rule 16 — Text length demotion:** full_address + median length >100 → sentence.
   - **Rule 17 — UTC offset override:** `[+-]HH:MM` at ≥80% → `datetime.offset.utc`. Between Rules 14 and 15.
   - **Rule 18 — Entity demotion:** full_name + entity classifier non-person >0.6 → entity_name. Fires before header hints. **Entity demotion guard:** skips header hints entirely when applied.
   - **Rule 19 — Percentage without '%' sign:** percentage winner + no values contain '%' → decimal_number. (NNFT-188)
5. **Validation-based candidate elimination** (NNFT-188): After vote aggregation, validates all top candidates against JSON Schema contracts. Eliminates candidates where >50% of sample values fail validation. Safety: keeps original votes if ALL eliminated. Runs before disambiguation.
6. **Header hints** (hardcoded first, then Model2Vec): Hardcoded `header_hint()` takes priority over Model2Vec semantic hints. Includes geography protection, measurement disambiguation, scientific measurement override (pressure/temperature/etc. → decimal_number), same-domain geo override (city↔country at ≤0.90).
7. **Geography rescue** (NNFT-188): When Sense misroutes location columns, checks unmasked CharCNN votes. Fires only when a location type is the plurality in unmasked distribution at ≥15%. Blocked by non-location, non-person header hints.
8. **Post-hoc locale detection:** Runs sample values against `validation_by_locale` patterns. Returns locale with highest pass rate >50%.
9. **`is_generic` determination:** Five additive signals — attractor-demoted, numeric_postal_code_detection, boolean, hardcoded list, taxonomy designation.

### Tiered model architecture

```
Tier 0 (root): DuckDB-type router (VARCHAR, BIGINT, DOUBLE, DATE, etc.)
  → Tier 1: Domain routers (VARCHAR → address/code/person/internet/...)
    → Tier 2: Leaf classifiers (VARCHAR_person → email/full_name/username/...)
```

34 specialised CharCNN models. Graph in `models/tiered-v2/tier_graph.json`.

### Taxonomy structure

Labels: `domain.category.type` (e.g., `identity.person.email`). 7 domains: container (12), datetime (45), finance (16), geography (15), identity (19), representation (32), technology (24).

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

### CLI commands

| Command | Purpose |
|---|---|
| `finetype infer` | Classify values (single/column/batch mode) |
| `finetype profile <file>` | Profile all columns in CSV/Parquet |
| `finetype check` | Validate taxonomy ↔ generator alignment |
| `finetype generate` | Generate synthetic training data |
| `finetype train` | Train CharCNN models (flat/tiered). `--seed N` for deterministic. Auto-snapshots. |
| `finetype taxonomy` | Print taxonomy summary (`--full --output json` for all fields) |
| `finetype schema <key>` | Export JSON Schema (`--pretty`, glob patterns) |

### Evaluation infrastructure

**Profile eval** (`eval/profile_eval.sh`) — 98.3% label (117/119), 99.2% domain (117/119) on 21 datasets.
**GitTables 1M** (`eval/gittables/`) — 47.1% label / 56.5% domain on format-detectable types.
**SOTAB CTA** (`eval/sotab/`) — 43.6% label / 68.6% domain on format-detectable types.
**Actionability eval** (`eval-actionability` binary) — 96.0% datetime format_string parse rate (2910/3030 values). Supports `format_string_alt` for type variants (e.g., ISO 8601 with/without fractional seconds). 2 columns below 95%: long_full_month_date (misclassification), multilingual.date (mixed formats).
**Precision per type** — Per-predicted-type precision: 🟢≥95%, 🟡80-95%, 🔴<80%.
**Dashboard:** `make eval-report` generates `eval/eval_output/report.md`.

All eval pipelines use `eval/config.env` + `envsubst` for dataset paths.

### Adding regression datasets

1. Create/extend CSV in `/home/hugh/datasets/` (~80 rows)
2. Add entries in `eval/datasets/manifest.csv` (dataset, file_path, column_name, gt_label)
3. Add schema mapping in `eval/schema_mapping.yaml` (match_quality: direct/close/partial)
4. `make eval-mapping` → `make eval-report` → verify

GT labels: lowercase with spaces. Current: 21 CSV files, 120 format-detectable columns.

## Priority Order

1. ✅ **Accuracy lift** — Completed NNFT-188: 117/119 (98.3% label). Remaining 2 ambiguous cases (countries.name, long_full_month_date) deferred to follow-up task.
2. **DuckDB extension metadata** — Replace last Python script in build chain (NNFT-183)
3. **Documentation** — README update, CHANGELOG (NNFT-095, NNFT-096)
4. **Distribution** — Homebrew tap, crates.io current, v0.5.1 release
5. **Optional: further actionability improvements** — At 96.0% (target met). Remaining gaps: long_full_month_date (misclassification), multilingual.date (mixed formats)

## Decided Items

Key decisions — do not revisit without good reason. See backlog decisions and task details for full context.

1. **Tiered model as default** — T0→T1→T2 for CLI; flat for DuckDB extension throughput. (NNFT-084/087/089)
2. **Taxonomy labels** — `domain.category.type` dotted hierarchy. Locale is a YAML field, not label. (NNFT-001)
3. **YAML transformation contracts** — Each type specifies DuckDB broad_type, transform SQL, validation. (NNFT-001)
4. **CharCNN via Candle** — Rust training and inference. No Python at runtime. (NNFT-003)
5. **Column-mode disambiguation** — Majority vote + hardcoded rules. Header hints override generic predictions. Geography protection + measurement disambiguation guards. (NNFT-065/091/102/127/128/156)
6. **Model2Vec semantic hints** — potion-base-4M, max-sim K=3 FPS matching, 0.65 threshold. Falls back to hardcoded `header_hint()`. (NNFT-110/122/124)
7. **Models on HuggingFace** — `hughcameron/finetype`. CI downloads via script. Not in git. (NNFT-020/088)
8. **Attractor demotion (Rule 15)** — Three signals: validation failure, confidence, cardinality. Locale-confirmed skips Signals 2-3. Universal validation can reject but cannot confirm. (NNFT-115/131/132)
9. **Duration override (Rule 14)** — SEDOL + P-prefix → duration. Before attractor demotion. (NNFT-131)
10. **JSON Schema validation** — `jsonschema` crate, Draft 2020-12. Pre-compiled validators cached. (NNFT-116)
11. **Locale-specific validation** — `validation_by_locale` for 5 types: postal_code (14 locales), phone_number (15), calling_code (17), month_name (6), day_of_week (6). Embedded in YAML. (NNFT-118/121/136/141)
12. **Validation precision** — For `locale_specific` types: locale validation confirms, universal validation can only reject. (NNFT-132)
13. **`is_generic` determination** — Five additive signals. Hardcoded list always applies; taxonomy designation adds more. (NNFT-139/156)
14. **Post-hoc locale detection** — Composable add-on after classification (decision-002 Option B). (NNFT-140/141)
15. **UTC offset override (Rule 17)** — `[+-]HH:MM` ≥80% → utc offset. Between Rules 14 and 15. (NNFT-143)
16. **Entity classifier (Rule 18)** — Deep Sets MLP (300→4 classes). Demotes full_name → entity_name when non-person >0.6. Entity demotion guard skips header hints. (NNFT-150-152, decision-003)
17. **Snapshot Learning** — Auto-snapshot before overwriting models. `--seed N` deterministic training. `manifest.json` provenance. (NNFT-146)
18. **Sense Architecture A** — Cross-attention over Model2Vec beats transformer encoder: +1.6pp accuracy, 23.7x faster, simpler Candle port. (NNFT-163, decision-005)
19. **Sense integration: flat CharCNN + output masking** — Use existing flat model with Sense-guided category masking, not per-category retraining. Sample 100/encode 50. Sense absorbs 6 behaviours (header hints, entity demotion, geography protection). (NNFT-164, decision-006)
20. **Pure Rust via Candle (Path A)** — Full Rust migration replacing all Python. Candle 0.8 with `half = "2.4"` pin. Validated: architecture, gradients, optimizer, safetensors round-trip. (NNFT-182/187)

## Build & Test

```bash
make setup              # Install git hooks (first time)
cargo build             # Build core, model, cli
cargo test              # Run test suite
cargo run -- check      # Validate taxonomy/generator alignment
make ci                 # fmt + clippy + test + check
cargo build -p finetype_duckdb --release  # DuckDB extension
make eval-report        # Profile eval + actionability + dashboard
```

## Key File Reference

| What | Where |
|---|---|
| Taxonomy definitions | `labels/definitions_*.yaml` (6 files) |
| Tiered model graph | `models/tiered-v2/tier_graph.json` |
| Column disambiguation | `crates/finetype-model/src/column.rs` |
| Semantic hint classifier | `crates/finetype-model/src/semantic.rs` |
| Entity classifier (Rust) | `crates/finetype-model/src/entity.rs` |
| Sense classifier (Rust) | `crates/finetype-model/src/sense.rs` |
| Shared Model2Vec resources | `crates/finetype-model/src/model2vec_shared.rs` |
| Label → category map | `crates/finetype-model/src/label_category_map.rs` |
| Model2Vec artifacts | `models/model2vec/` |
| Entity classifier model | `models/entity-classifier/` |
| DuckDB type mappings | `crates/finetype-duckdb/src/type_mapping.rs` |
| CLI entry point | `crates/finetype-cli/src/main.rs` |
| CI workflow | `.github/workflows/ci.yml` |
| Eval config | `eval/config.env` |
| Schema mapping | `eval/schema_mapping.yaml` |
| Eval binaries (report, actionability, GitTables, SOTAB) | `crates/finetype-eval/src/bin/` |
| Smoke tests | `tests/smoke.sh` |
| Phase 2 integration design | `discovery/architectural-pivot/PHASE2_DESIGN.md` |
| Architectural pivot | `discovery/architectural-pivot/` |
| Sense training (Rust) | `crates/finetype-train/src/sense_train.rs`, `crates/finetype-train/src/bin/train_sense.rs` |
| Entity training (Rust) | `crates/finetype-train/src/entity.rs`, `crates/finetype-train/src/bin/train_entity.rs` |
| Data pipeline (Rust) | `crates/finetype-train/src/data.rs`, `crates/finetype-train/src/bin/prepare_sense_data.rs` |
| Model2Vec prep (Rust) | `crates/finetype-train/src/model2vec_prep.rs`, `crates/finetype-train/src/bin/prepare_model2vec.rs` |
| Training infra (Rust) | `crates/finetype-train/src/training.rs` |
| Sense model artifacts | `models/sense/` (production), `models/sense_spike/arch_a/` (spike winner) |
| Sense A/B eval report | `eval/eval_output/sense_ab_diff.json` |
| Collapsed type remapping | `crates/finetype-model/src/column.rs` (search `remap_collapsed_label`) |
| Candle training spike | `crates/finetype-candle-spike/` (models, data, training, tests) |
| Candle spike summary | `discovery/candle-feasibility-spike/SUMMARY.md` |
| DuckDB metadata tool | `crates/finetype-build-tools/src/lib.rs`, `crates/finetype-build-tools/src/bin/append_duckdb_metadata.rs` |

## Backlog Discipline

**Every bug fix, feature, and release MUST have a corresponding backlog task.** Create retroactively with status `Done` if already complete.
