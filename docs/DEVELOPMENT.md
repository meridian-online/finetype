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
