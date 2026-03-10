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

**Version:** 0.6.8
**Taxonomy:** 250 definitions across 7 domains (container: 12, datetime: 84, finance: 31, geography: 25, identity: 34, representation: 36, technology: 28) — all generators pass, 100% alignment
**Default model:** Sense→Sharpen pipeline (CLI) with char-cnn-v14-250 flat (250 classes, 10 epochs, 372k samples), tiered-v2 fallback via `--sharp-only`. Hierarchical head available: char-cnn-v15-250 (7→43→250 tree softmax, 84.2% type / 90.9% domain / 96.5% category training accuracy).
**Features:** 34-dim deterministic feature extractor (NNFT-248/266), column-level aggregation (mean, variance, min, max), 4 feature-based disambiguation rules (F1–F4).
**Codebase:** ~20k lines of Rust across 9 crates (including finetype-train for pure Rust ML training, finetype-mcp for MCP server). Zero Python dependencies (build + runtime).
**CI status:** All checks pass (fmt, clippy, test, taxonomy check)
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged), MCP server (`finetype mcp`)

### Recent milestones

- **Sibling-context attention module** (NNFT-268, m-13) — 2-layer pre-norm transformer self-attention over Model2Vec column embeddings. Enriches per-column headers with cross-column context before Sense classification. 396,800 params (1.51 MB), 108μs–1.4ms latency. Architecturally complete but inert until trained — no model artifact means pipeline is unchanged (zero regression, 180/186 profile eval). Multi-column entry point: `classify_columns_with_context()`. Profile command uses batch path when sibling context available. Training data pipeline designed (GitTables). Addresses 3/7 remaining bare-name ambiguity errors.
- **Hierarchical classification head** (NNFT-267, m-13) — Tree softmax replacing flat 250-class output: 7 domains → 43 categories → 250 leaf types. HierarchyMap derived from label strings, HierarchicalHead with per-node Linear layers (39 non-degenerate leaf heads, 4 degenerate skipped). Multi-level CE loss (λ=0.2/0.3/0.5). CharCnn dual-mode (Flat/Hierarchical) with backbone_forward(). char-cnn-v15-250: 84.2% type, 90.9% domain, 96.5% category. Profile: 180/186 (96.8% label, 98.4% domain) — matches flat baseline. `--hierarchical` CLI/script flag. Backward compatible.
- **Column feature expansion** (NNFT-266, m-13) — FEATURE_DIM 32→34 (has_colon, has_dash). ColumnFeatures struct replaces raw mean array with mean/variance/min/max aggregation. Rule F4: zero length-variance + all hex + len=40 → git_sha (not hash). Rule F3 enhanced with float-parseability Path B. Profile: 180/186 (96.8% label, 98.4% domain). 1 fewer misclassification (git_sha fixed).
- **Deep accuracy spike** (NNFT-253/254, m-12) — NNFT-253 found feature_dim=32 regresses eval (-1.6pp city attractor). NNFT-254 expanded header hints (~30 new rules for epoch, age, altitude, categorical text, etc.), added cross-domain hardcoded hint override with domain-aware thresholds (0.85 cross-domain / 0.5 same-domain), fixed 7 substring matching bugs. Confirmed feature_dim=0 + rules is the better path. Profile: 179/186 (96.2% label, 98.4% domain). Actionability: 99.9%.
- **Feature-augmented CharCNN pipeline** (NNFT-247–251, m-12) — 32-feature deterministic extractor (parse tests, char stats, structural), parallel fusion at CharCNN classifier head (`feature_dim` config, backward compatible), Sense→Sharpen pipeline integration with per-value + aggregated column features, 3 feature-based disambiguation rules.
- **CharCNN v14 retrain for 250-type taxonomy** (NNFT-245) — Full pipeline retrain: CharCNN-v14-250 (250 classes, 10 epochs, 372k samples at 1500/type, 86.6% training accuracy), Sense retrained (87.1% broad, 78.5% entity), Model2Vec refreshed (750 embeddings × 128 dim). 5 new eval datasets covering all 43 new types (293 manifest entries). Profile: 140/189 columns (74.1% label, 81.0% domain) — expected regression from 43 new overlapping types. 3 new false positives: cpt/postal_code (5-digit overlap), hs_code/decimal_number, docker_ref/hostname. url/urn semantic proximity noted (hardcoded hint handles correctly). Default model symlink updated.
- **Taxonomy expansion to 250 types** (NNFT-244) — Added 43 new type definitions across all domains: geography +10 (wkt, geojson, h3, geohash, plus_code, dms, mgrs, iso6346, hs_code, unlocode), technology +11 (ulid, tsid, snowflake_id, aws_arn, s3_uri, jwt, docker_ref, git_sha, cidr, urn, data_uri), identity +15 (icd10, loinc, cpt, hcpcs, vin, eu_vat, ssn, ein, pan_india, abn, orcid, email_display, phone_e164, upc, isrc), finance +3 (figi, aba_routing, bsb), representation +4 (cas_number, inchi, smiles, color_hsl). Structural: `pii: Option<bool>` field on Definition struct (11 types tagged), `x-finetype-pii`/`x-finetype-transform-ext` in schema output, duration regex expanded to full ISO 8601 spec. Dedup: bcp47→locale_code alias, iso_8601_verbose→iso_8601 alias.
- **Taxonomy precision cleanup** (NNFT-242/243) — Removed 2 low-precision integer-range types (http_status_code, port — false positives on plain integers). Renamed 7 currency amount types from locale-based to format-structural names (amount_us→amount, amount_eu→amount_comma, amount_accounting_us→amount_accounting, amount_eu_suffix→amount_comma_suffix, amount_space_sep→amount_space, amount_indian→amount_lakh, amount_ch→amount_apostrophe). Old names preserved in aliases. 209→207 types.
- **MCP server** (NNFT-241) — `finetype mcp` subcommand exposing type inference to AI agents via Model Context Protocol. 6 tools (infer, profile, ddl, taxonomy, schema, generate) + taxonomy resources. Built on rmcp v1.1.0 (official Rust MCP SDK), stdio transport, JSON + markdown dual output. New `finetype-mcp` library crate.
- **Taxonomy cleanup** (NNFT-233/234) — Removed 7 low-precision types (216→209), recategorized color types, renamed 10 geographic type names to format-structural names (eu_→dmy_, us_→mdy_, american→mdy_12h, european→dmy_hm, decimal_number_eu→decimal_number_comma). CharCNN-v13 retrained on 209k samples (1000/type). Profile: 143/146 (97.9% label, 98.6% domain). Actionability: 99.3%.
- **Post-retrain accuracy recovery v13** (NNFT-235) — Five pipeline fixes for entity/geography confusion: (1) same-domain geo override ignores confidence threshold for hardcoded hints, (2) hardcoded person-name hints override location predictions, (3) 20+ entity-name header hints (company, venue, station, etc.), (4) bare "address" → full_address, (5) hardcoded hints apply at <0.5 confidence. Profile: 135/146→143/146 (97.9%). 3 remaining: bare "name" ambiguity.
- **Format Coverage expansion** (NNFT-222–226) — 53 new type definitions (163→216 types, 33% increase). 40 datetime + 13 finance formats including CJK dates, Apache CLF, ISO 8601 milliseconds, Indian lakh/crore, Swiss apostrophe, accounting notation. CharCNN-v12 retrained on 212k samples (1000/type). Pipeline fix: header-hint location override (Step 7b-pre) for Sense misrouting. Profile: 111/116 (95.7% label). Actionability: 96.2%.
- **Post-retrain accuracy recovery** (NNFT-194) — Five targeted pipeline fixes: (1) Rule 17 UTC offset guard removed (utc_offset fix), (2) rfc_2822/rfc_3339/sql_standard header hints added before generic timestamp catch-all, (3) full_address header hint distinguished from street_address, (4) same-category hardcoded hint override for within-category disambiguation, (5) enhanced geography protection checks unmasked votes at low confidence. Profile: 112/116→113/116 (97.4% label, 98.3% domain). Actionability: 95.4%→97.9%. 3 remaining misclassifications require model retrain.
- **Locale Foundation expansion** (NNFT-195–201) — Layer 1: Expanded validation to 50+ postal codes, 45+ phone numbers, 30+ month/day names. Layer 2: Expanded generators to match (65 postal locales, 46 phone locales, 32 CLDR date/time patterns). CharCNN-v11 retrained on expanded data (10 epochs, 88.3% training accuracy). Profile eval improved 110/116→112/116 (96.6%).
- **Taxonomy revision v0.5.2** (NNFT-192) — Removed `geography.address.street_number` (false positives on plain integers) and `identity.person.age` (indistinguishable from integer_number, 205 SOTAB false positives). Added `representation.identifier.numeric_code` (VARCHAR, preserves leading zeros for codes like ISO country numeric, NAICS, FIPS). Net: 164→163 types. CharCNN-v10 retrained. Actionability improved 96.0%→98.7%. Profile eval regressed 117/119→110/116 due to model retrain.
- **Actionability improvements** (NNFT-191) — Actionability 92.7% → 96.0% (2910/3030 values). Added `format_string_alt` field to taxonomy YAML for ISO 8601 fractional seconds variant. Updated eval to try multiple format strings per type. Fixed network_logs.timestamp (0% → 100%).
- **Accuracy improvements** (NNFT-188) — Profile eval 108/119 → 117/119 (98.3% label, 99.2% domain). Six mechanisms: validation-based candidate elimination, Rule 19, header hint additions, hardcoded hint priority over Model2Vec, same-domain geo override, geography rescue from unmasked votes.
- **v0.5.1 model retrain** (NNFT-181) — All models retrained on clean 164-type taxonomy. CharCNN-v9 (1,000 samples/type), refreshed Model2Vec type embeddings, Sense + Entity classifiers.
- **Pure Rust training** (NNFT-185) — All Python training scripts replaced with Rust/Candle. `finetype-train` crate with 4 binaries. Zero Python dependencies.
- **Taxonomy v0.5.1** (NNFT-177/178/179/180) — Finance domain (banking, commerce), identifier category. 164 types across 7 domains.

### What's in progress

- **Golden test expansion** (NNFT-258) — Rust integration tests covering profile, load, taxonomy, schema commands. Both small fixtures and real CSV datasets. Structured field matching (label, domain, confidence range). Depends on NNFT-254 completion.
- **Remaining accuracy gaps** — 6 misclassifications at 180/186: 3× bare "name" ambiguity (genuinely ambiguous), 2× model-level confusions (hs_code→decimal_number false positive on ecommerce totals, docker_ref→hostname), 1× GT edge case (response_time_ms integer vs decimal).

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
    +--- finetype-cli   (depends on core + model + mcp — CLI binary)
    +--- finetype-mcp   (depends on core + model — MCP server library)
    +--- finetype-duckdb (depends on core + model — DuckDB extension)

finetype-eval  (standalone — eval binaries, depends on csv/parquet/duckdb/arrow)
```

### Inference pipeline

**Value-level:** Single string → type label via `CharClassifier` (flat, 163 classes) or `TieredClassifier` (46 CharCNN models in T0→T1→T2 graph). Both implement `ValueClassifier` trait.

**Column-level (Sense→Sharpen, default):** Vector of strings + header → single column type:
0. **(Multi-column only, NNFT-268):** When sibling-context attention is loaded, encode all column headers → `[N_cols, 128]`, run self-attention → enriched `[N_cols, 128]`. Each column's enriched header is passed to Sense instead of raw Model2Vec encoding. Falls back to per-column when no model.
1. Sample 100 values, encode header + first 50 with Model2Vec
1b. **Extract deterministic features** for all sampled values (32-dim, NNFT-250). Compute aggregated column-level features (mean). Used for CharCNN augmentation and feature-based disambiguation.
2. Sense classify → broad category (temporal/numeric/geographic/entity/format/text) + entity subtype
3. Run flat CharCNN batch on all 100 values with per-value features (passed via `classify_batch_with_features`; ignored when model has `feature_dim=0`). Remap collapsed labels.
4. **Masked vote aggregation:** filter to category-eligible labels via `LabelCategoryMap`. Safety valve: falls back to unmasked when all votes filtered OR when Sense confidence <0.75 and masking removes >40% of votes
5. Apply disambiguation rules (same rules, votes already scoped). Coordinate disambiguation requires competitive vote share (prevents false-positive on decimal columns)
5b. **Feature-based disambiguation** (NNFT-250/266): Rule F1 (leading-zero → numeric_code for postal_code/cpt), Rule F2 (slash-segments → docker_ref), Rule F3 (digit-ratio+dots+float-parseability → hs_code), Rule F4 (zero length-variance + all hex + len=40 → git_sha)
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
| `finetype profile <file>` | Profile all columns in CSV/Parquet (`-o plain\|json\|csv\|markdown\|arrow`) |
| `finetype check` | Validate taxonomy ↔ generator alignment |
| `finetype generate` | Generate synthetic training data |
| `finetype train` | Train CharCNN models (flat/tiered). `--seed N` for deterministic. Auto-snapshots. |
| `finetype taxonomy` | Print taxonomy summary (`--full --output json` for all fields) |
| `finetype schema <key>` | Export JSON Schema (`--pretty`, glob patterns, `x-finetype-*` DDL fields) |
| `finetype load <file>` | Profile → runnable DuckDB CTAS (`--table-name`, `--limit N`, `--no-normalize-names`) |
| `finetype mcp` | Start MCP server over stdio (6 tools: profile, infer, ddl, taxonomy, schema, generate) |

### Evaluation infrastructure

**Profile eval** (`eval/profile_eval.sh`) — 96.8% label (180/186), 98.4% domain (183/186) on 30 datasets (293 manifest entries, 250-type taxonomy).
**GitTables 1M** (`eval/gittables/`) — 47.1% label / 56.5% domain on format-detectable types.
**SOTAB CTA** (`eval/sotab/`) — 43.6% label / 68.6% domain on format-detectable types.
**Actionability eval** (`eval-actionability` binary) — 99.9% transform success rate (232321/232541 values, 283 columns, 120 types). Supports `format_string_alt` for type variants (e.g., ISO 8601 with/without fractional seconds).
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
21. **MCP server via rmcp** — Official Rust MCP SDK v1.1.0, stdio transport, single binary (`finetype mcp` subcommand). 6 tools + taxonomy resources. JSON + markdown dual output. (NNFT-240/241)
22. **Rules over feature-augmented model** — feature_dim=0 + expanded header hints + F1-F4 post-vote rules outperforms feature_dim=32 CharCNN. Feature fusion causes city attractor regression (-1.6pp). Cross-domain hardcoded hint override with domain-aware thresholds (0.85 cross/0.5 same). Column-level variance/min/max aggregation (NNFT-266) for distributional disambiguation. (NNFT-253/254/266)
23. **Hierarchical classification head** — Tree softmax (7 domains → 43 categories → 250 leaf types) with multi-level CE loss (λ=0.2/0.3/0.5). CharCnn dual-mode: `new()` flat (default, backward compatible), `new_hierarchical()` tree. Product probabilities p(type)=p(domain)×p(cat|domain)×p(leaf|cat). Degenerate categories (1 type) skip leaf head. 84.2% type accuracy, maintains 180/186 profile eval. (NNFT-267)
24. **Sibling-context attention** — 2-layer pre-norm transformer self-attention (4 heads, 128-dim, 396K params) over Model2Vec column header embeddings. Enriches per-column headers with cross-column context before Sense. `classify_columns_with_context()` multi-column entry point; falls back to per-column when no model artifact. N=1 degrades gracefully via residual. Safetensors load/save. 108μs–1.4ms latency. Training requires GitTables multi-column table data. (NNFT-268)

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
| Taxonomy definitions | `labels/definitions_*.yaml` (6 files) |
| Tiered model graph | `models/tiered-v2/tier_graph.json` |
| Column disambiguation | `crates/finetype-model/src/column.rs` |
| Semantic hint classifier | `crates/finetype-model/src/semantic.rs` |
| Entity classifier (Rust) | `crates/finetype-model/src/entity.rs` |
| Sense classifier (Rust) | `crates/finetype-model/src/sense.rs` |
| Sibling-context attention | `crates/finetype-model/src/sibling_context.rs` |
| Shared Model2Vec resources | `crates/finetype-model/src/model2vec_shared.rs` |
| Label → category map | `crates/finetype-model/src/label_category_map.rs` |
| Model2Vec artifacts | `models/model2vec/` |
| Entity classifier model | `models/entity-classifier/` |
| DuckDB type mappings | `crates/finetype-duckdb/src/type_mapping.rs` |
| MCP server | `crates/finetype-mcp/src/lib.rs` |
| MCP tool handlers | `crates/finetype-mcp/src/tools/*.rs` (6 tools) |
| MCP taxonomy resources | `crates/finetype-mcp/src/resources.rs` |
| MCP spike report | `discovery/mcp-server/SPIKE.md` |
| CLI entry point | `crates/finetype-cli/src/main.rs` |
| CI workflow | `.github/workflows/ci.yml` |
| Eval config | `eval/config.env` |
| Schema mapping | `eval/schema_mapping.yaml` |
| Eval binaries (report, actionability, GitTables, SOTAB) | `crates/finetype-eval/src/bin/` |
| Smoke tests | `tests/smoke.sh` |
| Golden integration tests | `crates/finetype-cli/tests/cli_golden.rs` (13 tests, `#[ignore]`) |
| Test fixtures | `tests/fixtures/` (CSV + JSON fixtures) |
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
| Training script | `scripts/train.sh` |
| Eval script | `scripts/eval.sh` |
| Package script | `scripts/package.sh` |
| Device auto-detection (train) | `crates/finetype-train/src/device.rs` |

## Backlog Discipline

**Every bug fix, feature, and release MUST have a corresponding backlog task.** Create retroactively with status `Done` if already complete.

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
