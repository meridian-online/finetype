# FineType Development

## Runtime requirements

**The `duckdb` CLI is a hard runtime dependency (choice 0100).** `finetype profile` and `finetype validate` shell out to the external `duckdb` binary for all CSV and Parquet ingestion (and for validate's materialise/transform step). It must be on `PATH`; the commands fail with a single actionable error otherwise:

```
could not invoke duckdb CLI (is duckdb on PATH?): … Install it from https://duckdb.org/docs/installation
```

Install it via your platform package manager (`brew install duckdb`, etc.) or from <https://duckdb.org/docs/installation>. This is a *shell-out*, not a link — the cross-platform release build is unchanged (no `libduckdb` compile/link), so the Windows/MSVC amalgamation issue that sank choice 0099 cannot occur. Tested against duckdb v1.5.3.

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

The DuckDB extension is the in-tree workspace crate `finetype_duckdb` (`crates/finetype-duckdb/`). Its cdylib lib name is `finetype` (set in that crate's `Cargo.toml`), so the compiled library is `libfinetype.{dylib,so,dll}` and the loadable artifact is `finetype.duckdb_extension` — the package stays `finetype_duckdb` so `cargo build -p finetype_duckdb` is unchanged. A DuckDB metadata footer must be appended to the compiled library before it will load.

There are two build paths, both producing a `finetype.duckdb_extension` that loads unsigned (`duckdb -unsigned`):

### 1. Local quick build (pure Rust, no Python)

```bash
make build-release
```

This builds the cdylib, then appends metadata with the `finetype-build-tools` crate. It detects the platform (`.dylib` on macOS, `.so` on Linux) and stamps the **stable** C API (`--abi-type C_STRUCT`) at the `v1.2.0` floor, so one artifact loads on DuckDB 1.2 through 1.5+. The artifact lands at `target/release/finetype.duckdb_extension`. The metadata tool can also be run standalone:

```bash
cargo run -p finetype-build-tools --bin append-duckdb-metadata -- \
    -l target/release/libfinetype.dylib \
    -n finetype \
    -o target/release/finetype.duckdb_extension \
    -p osx_arm64 \
    --duckdb-version v1.2.0 \
    --extension-version 0.6.23 \
    --abi-type C_STRUCT
```

The metadata format follows DuckDB's extension specification: a custom section (`duckdb_signature`) carrying platform, version, and ABI type fields, plus 256 bytes reserved for signing.

### 2. Community-extensions build contract (extension-ci-tools)

This is the path `duckdb/community-extensions` CI uses when it rebuilds the registered extension. It is driven by the vendored `extension-ci-tools` submodule (pinned to `v1.5.3`) plus the `configure/` directory and the contract targets in the root `Makefile`. It needs Python 3 (for the venv + metadata script) and a Rust toolchain.

```bash
git submodule update --init --recursive   # first time only
make configure                            # creates configure/venv, detects platform + version
make release                              # builds finetype_duckdb, stamps metadata
# artifact: build/release/finetype.duckdb_extension
make test_release                         # runs test/sql/*.test via the DuckDB sqllogictest runner
```

The contract targets (`configure`, `release`, `debug`, `test_release`, `test_debug`) build the single workspace member `finetype_duckdb` — not the whole workspace — and copy the cdylib into `build/release/`. They include `extension-ci-tools/makefiles/c_api_extensions/base.Makefile` but **not** its `rust.Makefile`, because the stock rust.Makefile runs a whole-workspace `cargo build` that would pull in the heavy `finetype-train`/`finetype-eval` deps (bundled DuckDB, candle).

**Stable C API is the whole point.** `USE_UNSTABLE_C_API` is deliberately left unset, so metadata stamps the stable `C_STRUCT` ABI at `TARGET_DUCKDB_VERSION = v1.2.0`. The standalone repo used the *unstable* C API, which version-locks each artifact to one exact DuckDB release — that was the root cause of the community-channel 404 when DuckDB shipped 1.5.2/1.5.3. See choice 0063 (pin-strategy addendum).

## DuckDB Extension SQL Surface (`ft_` verbs)

The extension exposes a `profile → schema → validate` flow that mirrors the CLI, so an analyst answers "what type is this column?" and "is this table valid?" in SQL — not just "what type is this one value?". Every verb is `ft_`-prefixed; the older un-prefixed scalars (`finetype`, `finetype_detail`, `finetype_cast`, `finetype_unpack`, `finetype_validate`, `finetype_version`) stay registered as aliases for one release.

| Verb | Scope | DuckDB kind | Mirrors CLI |
|------|-------|-------------|-------------|
| `ft_infer(value)` | one value | scalar | — (weak probe) |
| `ft_profile(table)` | whole table | SQL table macro | `profile` |
| `ft_profile(list(col))` | a column | scalar over `LIST` | `profile` |
| `ft_validate_text(value, schema)` | one value / column | scalar → STRUCT | per-cell |
| `ft_validate(table, schema)` | a table | SQL table macro | `validate` |
| `ft_detail` / `ft_cast` / `ft_unpack` / `ft_version` | one value | scalar | utilities |

The two table verbs are symmetric — both take a table name: `ft_profile('t')` and `ft_validate('t', schema)`. Each is a SQL table macro registered at `LOAD`, so it reaches the catalog via `query_table(name)` past the `BindInfo` wall that blocks Rust table functions on this pin (choice 0064).

`ft_profile` is **one name covering two forms**, which DuckDB routes by call position:

- **`FROM ft_profile('t')`** — the table macro; one row per column `{column_name, type, confidence, duckdb_type}`. The everyday form.
- **`ft_profile(list(col))` in a `SELECT`** — the underlying scalar over a `LIST`, with a 2-arg `(LIST, header)` overload. The table macro calls this scalar internally; reach for it directly only when you've already assembled a list.

`ft_profile` ships as a scalar (plus a macro), not a true aggregate, because duckdb-rs `1.10503.1` exposes no aggregate-UDF registration API (the *aggregate wall*) — see choice 0064's 2026-06-03 addendum.

### Why `ft_profile`, not `ft_infer`, is the accurate path

The model is column-oriented (5-branch: char + embed + stats + header + validation, pooled across a column's values). A single value carries no column context, so `ft_infer(v)` is "profile with sample size 1" — strictly weaker. Reach for `ft_infer` only to probe one literal; use `ft_profile` over a column for a real answer.

```sql
LOAD 'finetype.duckdb_extension';

-- Single-value probe (no column context — weak):
SELECT ft_infer('jane.doe@company.co.uk');   -- identity.person.email

-- Profile a whole table — one row per column (the everyday form):
SELECT * FROM ft_profile('people');
-- ┌─────────────┬───────────────────────────────────────┬────────────┬─────────────┐
-- │ column_name │ type                                  │ confidence │ duckdb_type │
-- ├─────────────┼───────────────────────────────────────┼────────────┼─────────────┤
-- │ age         │ representation.numeric.integer_number │      0.956 │ BIGINT      │
-- │ email       │ identity.person.email                 │      1.000 │ VARCHAR     │
-- │ phone       │ identity.person.phone_number          │      0.500 │ VARCHAR     │
-- └─────────────┴───────────────────────────────────────┴────────────┴─────────────┘
-- The macro bakes in USING SAMPLE 100 ROWS, so it bounds the scan before
-- list() materialises a per-column array — safe on arbitrarily large tables.
```

### profile → schema → validate

Schema **generation** stays in the CLI (`finetype taxonomy`); the extension only **consumes** a JSON Schema. The one `schema` argument auto-detects inline JSON (`trim(schema) LIKE '{%'`), a `getvariable()` variable, or a file path — DuckDB short-circuits the `CASE` so the inline path never hits `read_text`.

```sql
-- 1. profile: see what each column is (CLI: finetype profile)
SELECT * FROM ft_profile('people');

-- 2. schema: generate a JSON Schema with the CLI, save to schema.json
--    finetype taxonomy ... > schema.json

-- 3. validate: is the loaded table valid? (CLI: finetype validate)
SELECT * FROM ft_validate('people', 'schema.json');
-- ┌─────────────┬───────┬─────────┬───────────────────────────────────────┐
-- │ column_name │ total │ rejects │            sample_message             │
-- ├─────────────┼───────┼─────────┼───────────────────────────────────────┤
-- │ email       │     3 │       1 │ "not-an-email" does not match "^…$"    │
-- │ phone       │     3 │       1 │ "not-a-phone" does not match "^…$"     │
-- └─────────────┴───────┴─────────┴───────────────────────────────────────┘

-- Inline schema works through the same one argument (no file-not-found):
SELECT * FROM ft_validate('people',
  '{"properties":{"email":{"type":"string","pattern":"^[^@]+@[^@]+\\.[^@]+$"}}}');
```

`ft_validate_text` is the per-cell verb behind the macro — it returns a STRUCT naming which constraint failed (mirroring the CLI `reject_errors` shape), and applies the CLI null semantics (empty / `"null"` skip):

```sql
SELECT ft_validate_text('not-an-email',
  '{"type":"string","pattern":"^[^@]+@[^@]+\\.[^@]+$"}');
-- {'valid': false, 'constraint': pattern, 'message': '"not-an-email" does not match …'}
```

> **Note the JSON escaping.** A regex `\.` must be written `\\.` in the schema string — `\.` is invalid JSON. DuckDB single-quoted strings pass backslashes through literally, so `'…\\.[^@]…'` reaches the validator as valid JSON.

### Nested-column guard (Precision Principle)

`COLUMNS(*)::VARCHAR` flattens a STRUCT / LIST to DuckDB **display** text (`{'city': London}` — inner quotes lost, not valid JSON), which would silently FALSE-reject. Both table macros guard nested columns: `ft_validate` and `ft_profile` surface them as an explicit skip row (NULL counts) telling the analyst to `unnest` / `to_json` / extract first, rather than reporting bogus rejects or profiling display text.

```
│ addr │ NULL │ NULL │ nested STRUCT(city VARCHAR) column — unnest / to_json / extract before validating │
```

> The community `description.yml` (`extended_description` / `hello_world`) teaches the `ft_` profile → schema → validate surface as of the v0.6.23 republish; the un-prefixed scalars (`finetype`, `finetype_validate`, …) remain registered as aliases for one release so a v0.6.22 install keeps working.

## Related Repositories

- **meridian-online/finetype** (this repo) — Production codebase. Candle-based, DuckDB integration.
- **hughcameron/finetype** — v1 experiments. Burn+LibTorch training, Python data generation with mimesis.

---

## CLI Surface (v0.6.23)

`finetype --help` lists **only the 5 public commands**. Hidden subcommands stay callable for internal use (CI, training data prep, sweep wrappers, eval scripts) but never appear in the help surface — they're not part of the stable contract and may move or change shape between minor versions without a deprecation cycle.

```
| Tier            | Commands                                                       |
|-----------------|----------------------------------------------------------------|
| Public (v0.6.23)| `infer`, `profile`, `validate`, `mcp`, `taxonomy`              |
| Internal (hidden)| `check`, `generate`, `train`, `train-multi-branch`*,          |
|                  | `eval`, `infer-batch`                                         |
```

\* `train-multi-branch` is feature-gated behind `train` (or `cuda`/`metal`,
which enable it transitively). It is the only command that links
`finetype-train` — which bundles DuckDB's C++ — so the distributed `cpu`
binary omits it, keeping releases lean and dodging the Windows MSVC failure
that bundled DuckDB 1.5.3 triggers. Train from source with
`cargo run --features train -- train-multi-branch ...`; the GPU training
scripts already pass `--features metal`/`cuda`, so they get it automatically.

The hide mechanism is `#[command(hide = true)]` on the clap variant — no wrapper scripts, no env-var gating, no separate binary. Hidden ≠ removed: `finetype check` continues to power `make ci` and `finetype generate` continues to power training data prep.

`--model` is no longer a subcommand flag. The model directory is configured exclusively via the `FINETYPE_MODEL` env var (default: `models/default`).

### Commands

```
| Command | Purpose |
|---|---|
| `finetype infer` | Classify values (single/column/batch mode) |
| `finetype profile <file>` | Profile all columns in CSV/Parquet (`-o plain\|json\|csv\|markdown\|arrow`, `--enum-threshold N`, `--verbose`). Emits **`x-finetype-enum`** per column — the observed OPEN bounded domain `{ open, domain, distinct, rows, cohesion }` for any non-denylisted column whose full-column cardinality is bounded (`distinct ≤ 32`, `distinct/rows ≤ 0.5`), **decoupled from the label** (a `country_code` column gets its domain too; choice 0102). It is DESCRIPTIVE — validators ignore `x-finetype-*`, so it never constrains `validate`; the closed validation-enforced `enum` keyword stays conservative (categorical/boolean). Present in `-o json` and `-o json-schema --stats`; the CLI and MCP share one policy (`finetype_core::enum_domain`). |
| `finetype taxonomy [KEY]` | Print taxonomy summary, or filter to a single type / glob (`KEY` = `identity.person.email` or `identity.person.*`). Output formats: `-o plain\|json\|csv\|json-schema`. Per-type JSON Schema export (formerly the `schema KEY` verb) lives here; output is always a JSON array even for single matches. Schema export carries only `x-finetype-label` + `x-finetype-pii` extensions. |
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

## Build & Test (v0.6.23)

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
