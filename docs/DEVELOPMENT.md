# FineType Development

## Training (Pure Rust)

All model training uses the `finetype-train` crate. No Python required.

### Prerequisites

- SOTAB CTA data at `~/datasets/sotab/cta/` (validation + test splits)
- Model2Vec artifacts at `models/model2vec/` (model.safetensors, tokenizer.json)
- Profile eval datasets listed in `eval/datasets/manifest.csv`

### Full training pipeline

```bash
# 1. Prepare training data (SOTAB + profile eval + synthetic headers)
make train-prepare-sense

# 2. Generate Model2Vec type embeddings from taxonomy
make train-prepare-model2vec

# 3. Train Sense classifier (cross-attention over Model2Vec)
make train-sense

# 4. Train Entity classifier (Deep Sets MLP)
make train-entity

# Or run everything:
make train-all
```

### Individual binaries

```bash
# Data preparation with custom options
cargo run --release -p finetype-train --bin prepare-sense-data -- \
    --sotab-dir ~/datasets/sotab/cta \
    --output data/sense_prod \
    --include-profile \
    --synthetic-headers \
    --header-fraction 0.5 \
    --val-fraction 0.2

# Sense model training with custom hyperparameters
cargo run --release -p finetype-train --bin train-sense-model -- \
    --data data/sense_prod \
    --output models/sense_prod/arch_a \
    --epochs 50 \
    --batch-size 64 \
    --lr 5e-4 \
    --patience 10 \
    --header-dropout 0.5

# Entity classifier training
cargo run --release -p finetype-train --bin train-entity-classifier -- \
    --sotab-dir ~/datasets/sotab/cta \
    --model2vec-dir models/model2vec \
    --output models/entity-classifier

# Model2Vec type embedding generation
cargo run --release -p finetype-train --bin prepare-model2vec -- \
    --labels-dir labels \
    --model2vec-dir models/model2vec \
    --output models/model2vec
```

### Validation

After training, verify accuracy on profile eval:

```bash
make eval-report
```

Target: ≥170/174 label accuracy (97.7%).

### Architecture

- **Sense model (Architecture A):** Cross-attention over Model2Vec embeddings. Dual heads: broad category (6 classes) + entity subtype (4 classes). ~347k parameters.
- **Entity classifier:** Deep Sets MLP with 300-dim features (256 Model2Vec + 44 statistical). 4 entity classes. Demotion threshold configurable.
- **Data pipeline:** SOTAB parquet → DuckDB → frequency-weighted sampling → Model2Vec encoding → JSONL with pre-computed embeddings.

### Crate structure

```
crates/finetype-train/
    src/
        lib.rs              # Module declarations
        sense.rs            # SenseModelA architecture
        sense_train.rs      # Sense training loop
        entity.rs           # Entity classifier + training
        training.rs         # Shared infrastructure (loss, scheduler, early stopping)
        data.rs             # Data loading, SOTAB integration, JSONL pipeline
        model2vec_prep.rs   # FPS algorithm, type embedding generation
    src/bin/
        train_sense_model.rs      # CLI: train-sense-model
        train_entity_classifier.rs # CLI: train-entity-classifier
        prepare_sense_data.rs     # CLI: prepare-sense-data
        prepare_model2vec.rs      # CLI: prepare-model2vec
```

## DuckDB Extension Build

The DuckDB extension requires metadata appended to the compiled shared library. This is handled by the `finetype-build-tools` crate.

```bash
# Full release build (includes metadata appending)
make build-release

# The metadata tool can also be used standalone:
cargo run -p finetype-build-tools --bin append-duckdb-metadata -- \
    -l target/release/libfinetype_duckdb.so \
    -n finetype_duckdb \
    -o target/release/finetype_duckdb.duckdb_extension \
    -p linux_amd64 \
    --duckdb-version v1.2.0 \
    --extension-version 0.5.1 \
    --abi-type C_STRUCT
```

The metadata format follows DuckDB's extension specification: a WebAssembly custom section (`duckdb_signature`) containing platform, version, and ABI type fields, plus 256 bytes reserved for signing.

If the build tool is unavailable, `make build-release` falls back to copying the raw `.so` without metadata (the extension will load with `-unsigned` flag only).

## Related Repositories

- **meridian-online/finetype** (this repo) — Production codebase. Candle-based, DuckDB integration.
- **hughcameron/finetype** — v1 experiments. Burn+LibTorch training, Python data generation with mimesis.

---

## CLI Surface (v0.6.19)

`finetype --help` lists **only the 5 public commands**. Hidden subcommands stay callable for internal use (CI, training data prep, sweep wrappers, eval scripts) but never appear in the help surface — they're not part of the stable contract and may move or change shape between minor versions without a deprecation cycle.

```
| Tier            | Commands                                                       |
|-----------------|----------------------------------------------------------------|
| Public (v0.6.19)| `infer`, `profile`, `validate`, `mcp`, `taxonomy`              |
| Internal (hidden)| `check`, `generate`, `train`, `train-multi-branch`,           |
|                  | `eval`, `infer-batch`                                         |
```

The hide mechanism is `#[command(hide = true)]` on the clap variant — no wrapper scripts, no env-var gating, no separate binary. Hidden ≠ removed: `finetype check` continues to power `make ci` and `finetype generate` continues to power training data prep.

`--model` was removed from every subcommand in v0.6.19. The model directory is now configured exclusively via the `FINETYPE_MODEL` env var (default: `models/default`).

### Commands

```
| Command | Purpose |
|---|---|
| `finetype infer` | Classify values (single/column/batch mode) |
| `finetype profile <file>` | Profile all columns in CSV/Parquet (`-o plain\|json\|csv\|markdown\|arrow`, `--enum-threshold N`, `--verbose`) |
| `finetype taxonomy [KEY]` | Print taxonomy summary, or filter to a single type / glob (`KEY` = `identity.person.email` or `identity.person.*`). Output formats: `-o plain\|json\|csv\|json-schema`. Per-type JSON Schema export (formerly the `schema KEY` verb) lives here in v0.6.19+; output is always a JSON array even for single matches. Schema export carries only `x-finetype-label` + `x-finetype-pii` extensions. |
| `finetype validate <file> <schema>` | Schema-driven quality gate. Defaults to **check-only mode** — runs the validation engine, prints a summary, exits 0/1/2 (no rejects / rejects / error). Pass `--db <out.db> --table <name>` to also materialise the user's **typed** table (per-column transforms applied via TRY-wrapped projection from each column's `x-finetype-label`) plus `finetype_reject_errors` sidecar (13-col DuckDB `reject_errors` shape + FineType extensions `type_confidence`, `expected_type`, `constraint_failed`, `constraint_value`). Reject ontology: `error_type='SEMANTIC_TYPE'` for engine validation failures; `error_type='TRANSFORM_FAILED'` (`constraint_failed='transform'`) for cells that passed validation but failed the typed cast. Staging-NULL → typed-NULL is not a transform failure. ENUM emission is dropped — low-cardinality columns retain the schema's `duckdb_type`. Flags: `--append` (reuse db, scan_id++; requires `--db`), `--lenient` (force exit 0). Materialise mode requires `duckdb` on PATH. See MADR 0064 (reject pipeline) and MADR 0071 (load fold). |
| `finetype mcp` | Start MCP server over stdio (8 tools). |
| `finetype check` *(hidden)* | Validate taxonomy ↔ generator alignment. Used by `make ci`. |
| `finetype generate` *(hidden)* | Generate synthetic training data. Used by training data prep. |
| `finetype train` *(hidden)* | Train CharCNN models (flat/tiered). `--seed N` for deterministic. Auto-snapshots. |
```

## Model-Name Env Vars

Three env vars exist — each is read by exactly one consumer. Do not conflate.

```
| Env var              | Consumer                           | Purpose                                     |
|----------------------|------------------------------------|---------------------------------------------|
| FINETYPE_CI_MODEL    | .github/scripts/download-model.sh  | CI's authoritative model name for fetches   |
| FINETYPE_MODEL       | CLI, eval scripts                  | Model directory path (default: `models/default`) |
| FINETYPE_MODEL_DIR   | DuckDB extension                   | Local path override (bypasses HF download)  |
```

CLI/MCP/DuckDB/eval code does NOT read `FINETYPE_CI_MODEL`. The runtime default remains `models/default` for every non-CI consumer.

## Build & Test (v0.6.19)

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
