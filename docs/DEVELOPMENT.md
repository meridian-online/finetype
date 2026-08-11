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

The extension exposes a `profile → schema → validate` flow that mirrors the CLI, so an analyst answers "what type is this column?" and "is this table valid?" in SQL — not just "what type is this one value?". Every verb is `ft_`-prefixed.

**Breaking change: the un-prefixed scalars are gone.** `finetype`, `finetype_detail`, `finetype_cast`, `finetype_unpack`, `finetype_validate` and `finetype_version` were deprecated in 0.6.23 and are no longer registered, so a call to one now raises `Catalog Error`. Two of the six do not map by renaming — `finetype` was column-level and maps to `ft_profile`, not to the single-value `ft_infer`, and `finetype_validate` maps to `ft_validate_text`, which returns a `STRUCT` where the old scalar returned a `VARCHAR`. The migration table is in [CHANGELOG.md](../CHANGELOG.md).

| Verb | Scope | DuckDB kind | Mirrors CLI |
|------|-------|-------------|-------------|
| `ft_infer(value)` | one value | scalar | — (weak probe) |
| `ft_profile(table)` | whole table | SQL table macro | `profile` |
| `ft_profile(col)` / `ft_profile(col, header)` | a column | aggregate | `profile` |
| `ft_validate_text(value, schema)` | one value / column | scalar → STRUCT | per-cell |
| `ft_validate(table, schema)` | a table | SQL table macro | `validate` |
| `ft_detail` / `ft_cast` / `ft_unpack` / `ft_version` | one value | scalar | utilities |

The two table verbs are symmetric — both take a table name: `ft_profile('t')` and `ft_validate('t', schema)`. Each is a SQL table macro registered at `LOAD`, so it reaches the catalog via `query_table(name)` past the `BindInfo` wall that blocks Rust table functions on this pin (choice 0064).

### One name per granularity

The surface is organised by *what you hand it*, not by how it is implemented:

| You have | Call | Kind |
|---|---|---|
| one value | `ft_infer('alice@example.com')` | scalar |
| one column | `ft_profile(email)` | aggregate |
| one column, and a name for it | `ft_profile(val, col)` with a `GROUP BY col` | aggregate |
| a whole table | `FROM ft_profile('people')` | table macro |

`ft_profile` is **one name covering both column forms and the table form**, which DuckDB routes by call position: a call in `FROM` binds the table macro, a call in a projection binds the aggregate.

The header hint is the second argument. It feeds the model's header branch, which is why the table macro passes each column's own name: profiling a column called `email` is a different question from profiling the same values with no name attached.

**An aggregate-level `ORDER BY` is not supported.** `ft_profile(col ORDER BY col)` reads out of bounds inside DuckDB. The fault is in the C API's shared update path, not in this aggregate: the sorted path makes the state vector constant, and `CAPIAggregateUpdate` flattens the input vectors without flattening the state — its `combine` and `finalize` siblings both do. It is reported upstream and deferred. Order the statement instead.

`ft_profile` reached this shape by giving up a scalar. It was `ft_profile(list(col))`, a scalar over an assembled `LIST`, because duckdb-rs exposes no aggregate-UDF registration API. DuckDB's C API does, and the extension already talks to it directly — but an aggregate cannot be registered at a name a scalar already holds, so the `list()` forms retired to free the name. `ft_profile(list(col))` becomes `ft_profile(col)`, and `ft_profile(list(col), h)` becomes `ft_profile(col, h)`.

### Why `ft_profile`, not `ft_infer`, is the accurate path

The model is column-oriented (5-branch: char + embed + stats + header + validation, pooled across a column's values). A single value carries no column context, so `ft_infer(v)` is "profile with sample size 1" — strictly weaker. Reach for `ft_infer` only to probe one literal; use `ft_profile` over a column for a real answer.

```sql
LOAD './target/release/finetype.duckdb_extension';

-- Single-value probe (no column context — weak):
SELECT ft_infer('jane.doe@company.co.uk');   -- identity.person.email

-- Profile ONE column. The aggregate pools the column's values into a single
-- sample and returns one row:
SELECT ft_profile(email) FROM people;
-- {'type': identity.person.email, 'confidence': 0.502, 'duckdb_type': VARCHAR}

-- With a header hint, which feeds the model's header branch:
SELECT ft_profile(email, 'email') FROM people;
-- {'type': identity.person.email, 'confidence': 0.677, 'duckdb_type': VARCHAR}

-- It is a real aggregate, so GROUP BY works and each group is profiled on its
-- own sample. One row per group, each carrying that group's own STRUCT.
SELECT age > 30 AS older, ft_profile(email) AS profile
FROM people GROUP BY older ORDER BY older;
-- ┌─────────┬─────────┐
-- │  older  │ profile │
-- ├─────────┼─────────┤
-- │ false   │ {…}     │
-- │ true    │ {…}     │
-- └─────────┴─────────┘

-- Profile a whole table — one row per column (the everyday form):
SELECT * FROM ft_profile('people');
-- ┌─────────────┬───────────────────────────────────────┬────────────┬─────────────┐
-- │ column_name │ type                                  │ confidence │ duckdb_type │
-- ├─────────────┼───────────────────────────────────────┼────────────┼─────────────┤
-- │ age         │ representation.numeric.integer_number │      0.956 │ BIGINT      │
-- │ email       │ identity.person.email                 │      1.000 │ VARCHAR     │
-- │ phone       │ identity.person.phone_number          │      0.500 │ VARCHAR     │
-- └─────────────┴───────────────────────────────────────┴────────────┴─────────────┘
-- The macro bakes in USING SAMPLE 100 ROWS, so it reads a bounded number of
-- rows rather than the whole table before grouping them by column.
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

> The community `description.yml` (`extended_description` / `hello_world`) teaches the `ft_` profile → schema → validate surface as of the v0.6.23 republish. The un-prefixed scalars it superseded are no longer registered, so the community entry has to be republished off a build that no longer carries them before an install can be told the old names are gone.

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
| `finetype profile <file>` | Profile all columns in CSV/Parquet (`-o plain\|json\|csv\|markdown\|arrow\|json-schema\|datapackage`, `--enum-threshold N`, `--verbose`). Emits **`x-finetype-enum`** per column — the observed OPEN bounded domain `{ open, domain, distinct, rows, cohesion }` for any non-denylisted column whose full-column cardinality is bounded (`distinct ≤ 32`, `distinct/rows ≤ 0.5`), **decoupled from the label** (a `country_code` column gets its domain too; choice 0102). It is DESCRIPTIVE — validators ignore `x-finetype-*`, so it never constrains `validate`; the closed validation-enforced `enum` keyword stays conservative (categorical/boolean). Present in `-o json` and `-o json-schema --stats`; the CLI and MCP share one policy (`finetype_core::enum_domain`). **`-o datapackage`** (choice 0105) emits a conformant Frictionless **Data Package** descriptor — one Data Resource (`name`/`path`/`format`/`mediatype`/`encoding`/`bytes`/`sha256` hash) wrapping a Table Schema whose field `type`/`format` come from the authoritative per-leaf `frictionless:` map in the taxonomy (the canonical 244→16 fold FineType owns for the Meridian family; dovetail/arcform consume it via `finetype_core::frictionless_for`). Constraints come from the type's validation; FineType richness rides as `x-finetype-*` custom properties (`label`/`confidence`/`pii`/`locale`/`enum-domain`). `$schema` pins the v2.0 profile (vendored at `vendor/frictionless/`); descriptors validate against it (conformance test in `finetype-mcp/tests/conformance.rs`). Additive — the `json-schema` output is unchanged; the executable DuckDB `transform` is deliberately NOT in the descriptor (a Data Package *describes*, it does not *execute*). |
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

## Documentation gates

Three checks fail the build when the documentation stops agreeing with the code.
Run them all with `make check-docs`. Each derives its answer from the artefact
rather than from a second copy of the prose, and each ships a `--self-test` that
mutates a scratch tree and requires the gate to redden — a gate that is only
known to pass is not known to detect.

| Gate | Derives from | Catches | CI job |
|---|---|---|---|
| `scripts/check_doc_taxonomy_counts.py` | `labels/definitions_*.yaml`, via the `include_str!` list the build embeds | Every documented type, domain and locale count, headline and table row alike | `evidence` |
| `scripts/check_duckdb_catalog.py` | `duckdb_functions()` of a **loaded local build** | Function names, kinds and return types — the STRUCT-documented-as-VARCHAR family, and any documented call to a function nobody registered | `doc-surface` |
| `scripts/check_sql_examples.py` | running every ```sql fence against that build | An example that does not run, a result box naming a column the query does not return, a JSON key that is never emitted, a `LOAD` of an artifact no build produces | `doc-surface` |

`tests/doc_tests.sh` is **not** one of them — it exits 0 unconditionally and its
own banner says so.

The two catalog gates need `make build-extension` (the cdylib plus its metadata
stamp — not the whole workspace) and the `duckdb` CLI. `check_sql_examples.py`
additionally needs a model: it uses `FINETYPE_MODEL_DIR` if set, otherwise
`models/default`.

**What the SQL gate deliberately does not assert:** no confidence digit, no
predicted label, no score, no row count. Those move with a retrain and pinning
one would make the gate flake and then be ignored. Its self-test proves the point
by moving a confidence digit and a predicted label and requiring the gate to stay
green.

## Which self-tests a pull request runs

Every gate here ships a `--self-test` — a harness that mutates the gate, or the
tree the gate reads, and requires the gate to redden. Those proofs are routed:
`.github/gate-self-tests.tsv` says which paths invalidate which proof, and
`.github/scripts/gate-self-tests.py` diffs the pull request against its base and
publishes one boolean per gate that each self-test step in
`.github/workflows/ci.yml` is guarded by. A diff that leaves the gates alone runs
none of them; a diff that rewrites one runs that one.

```bash
make check-gate-routing     # the audit, and the router's own self-test
```

**Adding a gate means adding a row.** The audit runs on every pull request and
reddens when the manifest, the tree and the workflow disagree — a script carrying
a `--self-test` that no row watches, a registered command no step runs, or a step
that runs unguarded and so runs on every diff.

**A guard is compared whole, and the reason is worth reading once.** A routed
step carries exactly `steps.<routing step>.outputs.<id> == 'true'`. Anything else
is refused, because the near misses are not typos and do not look like defects:
`== 'false'` and `!= 'true'` invert the routing so the proof runs only when the
gate did *not* change, and an appended `&& github.event_name == 'push'` stops it
running on pull requests at all. Each still mentions the right output, so a check
that looked for the output name rather than the whole expression accepted all
three.

**Each job routes itself, and that is not a style choice.** A shared routing job
that other jobs `needs:` was measured on a probe pull request: when it failed, its
dependants were *skipped*, and a required status check that is skipped **satisfies
branch protection** — the pull request reported `UNSTABLE`, which is mergeable,
not `BLOCKED`. Two of this repository's five required contexts were among them. So
no job depends on another for routing, and `plan` runs the audit before it plans,
which puts the audit inside required contexts where a failure actually blocks.

Three further shapes are refused because they are *silent* — the step is skipped
and the job is green, with nothing to read: a guard naming a step that does not
exist, a guard sitting above the step that sets it (Actions resolves
`steps.<id>.outputs` against what has already run), and a job-level `if:` on a job
holding a proof. `--self-test` carries a named case for every shape named in these
three paragraphs.

**Everything uncertain routes more work, never less.** No base commit, an
unfetchable one, a diff that will not run, or a change to the manifest, the router
or `.github/workflows/ci.yml` each select every gate and say so in the log.
