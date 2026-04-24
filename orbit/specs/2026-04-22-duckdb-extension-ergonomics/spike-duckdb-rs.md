# Spike: duckdb-rs 1.4.4 VTab feasibility

**Date:** 2026-04-24
**Investigator:** Nightingale
**Spec:** `orbit/specs/2026-04-22-duckdb-extension-ergonomics/spec.yaml` (ac-04)
**Branch:** `validate-as-duckdb-reject-pipeline`

## Scope

The DuckDB-native reject pipeline (spec ac-05) requires a table function
`finetype_validate(table_name, schema_path)` that:

1. Binds with two VARCHAR parameters
2. At bind time, derives an output schema from the source table named in
   `table_name`
3. At execution time, reads rows from that source table, validates each
   against the JSON Schema at `schema_path`, and emits an annotated
   relation (original columns + `_row_index`, `_is_valid`, `_reject_info`,
   `_reject_column`)

The spike asks whether duckdb-rs 1.4.4 supports this shape.

## Findings

### (a) Is `vtab` available under `loadable-extension`?

**YES — no Cargo changes required.**

The duckdb-rs 1.4.4 feature graph, from its own `Cargo.toml` lines 66–70:

```toml
loadable-extension = [
    "vtab",
    "duckdb-loadable-macros",
    "libduckdb-sys/loadable-extension",
]
```

The `loadable-extension` feature implicitly activates `vtab`. The
workspace declaration in the project root `Cargo.toml` already enables
`loadable-extension`:

```toml
duckdb = { version = "=1.4.4", features = ["loadable-extension", "vscalar"] }
```

So `vtab` is already usable from the `finetype_duckdb` crate without
any feature-list edit. The spec's ac-04 assumption that "vtab feature
must be enabled" was incorrect — it is already on by transitive activation.

**Evidence:** `crates/finetype-duckdb/src/spike.rs` defines
`FineTypeSpike: VTab` using only types from `duckdb::vtab`. `lib.rs`
registers it via `con.register_table_function::<FineTypeSpike>("finetype_spike")`.
`cargo build -p finetype_duckdb` succeeds clean.

### (b) Can a table function share a catalog name with an existing scalar function?

**COMPILE-PROVEN; RUNTIME-UNPROVEN but precedent is favourable.**

A registration-site call to both `register_scalar_function::<S>("finetype_validate")`
and `register_table_function::<T>("finetype_validate")` in the same
entrypoint compiles cleanly. The Rust-level API permits it — both
methods return `Result<()>` and do not statically conflict.

**DuckDB precedent:** native `range` is both a scalar (`SELECT range(5)`
returns a LIST) and a table function (`SELECT * FROM range(5)` returns
a relation). DuckDB's catalog separates scalar and table function
namespaces and disambiguates by syntactic position.

**What was not proven:** a full end-to-end load of the built
`.duckdb_extension` artefact through a DuckDB CLI binary matching
libduckdb-sys 1.4.4, with both forms queried in the same session. The
locally-installed DuckDB CLI is v1.5.2, which refuses to load the
1.4.4-compiled dylib. Runtime proof is deferred to ac-07's integration
test, which uses duckdb-rs's own `Connection` from the same crate and
therefore matches the ABI exactly.

**Risk assessment:** LOW. DuckDB's catalog routinely supports this
pattern for built-in functions. The Rust wrapper does not impose
constraints beyond what DuckDB itself enforces. If runtime proof reveals
otherwise, rollback-plan Scenario B is unambiguous: rename to
`finetype_validate_rows`.

### (c) Can a VTab derive its output schema from a runtime table name?

**NO — blocked in the safe Rust API.**

This is the spike's critical negative finding. The VTab model in
duckdb-rs 1.4.4 is a **data-generation** contract, not a
**relational-operator-over-existing-tables** contract.

**What BindInfo exposes** (`duckdb-1.4.4/src/vtab/function.rs`):
- `get_parameter(i)` / `get_named_parameter(name)` — read parameters
  the caller passed
- `add_result_column(name, type)` — declare the output schema
- `set_bind_data(...)` — store state for later stages
- `get_extra_info::<T>()` — retrieve compile-time-registered extra data

**What it does NOT expose:**
- No `Connection` handle
- No catalog-lookup API (`DESCRIBE`, `information_schema.columns`)
- No row-iteration API over a named table

**What the VTab trait's `func()` is supposed to do:** generate rows
from its own `BindData` + `InitData` + incremental state. The built-in
examples (`hello`, `range`) illustrate this: the function writes rows
to a `DataChunkHandle` using data it already holds. There is no
primitive for "read from another DuckDB table and stream it through."

**Examined workarounds and their verdicts:**

1. **`register_table_function_with_extra_info<T, E>` + stash a `Connection`**
   — blocked. `E` must be `Send + Sync + 'static`. `duckdb::Connection`
   is not `Sync`; and even if a wrapper were `Send + Sync`, calling back
   into DuckDB from within a bind callback while the caller's statement
   is still in bind is an undefined pattern (potential catalog-lock
   re-entry).
2. **Direct C API calls via `libduckdb-sys`** (`duckdb_client_context`,
   `duckdb_connection_from_context`, etc.) — theoretically possible but
   pulls all memory-safety discipline into our own code. Not justifiable
   for ac-04 scope; reserved as a future option if the pattern becomes
   strategic.
3. **Take a file path instead of a table name** — violates spec
   constraint 3 ("Table function input is a table name string, not an
   arbitrary relation").
4. **Replacement scan API** — not exposed in duckdb-rs 1.4.4's public
   surface at all.

**Evidence:** grep of `duckdb-1.4.4/src/vtab/` for `Connection`,
`client_context`, `duckdb_client` returns zero matches in the public
types (`BindInfo`, `InitInfo`, `TableFunctionInfo`). The only types
accessed are caller-supplied parameters and output chunks.

## Verdict

The spec's ac-05 shape — a `finetype_validate(table_name, schema.json)`
table function that reads the named table and returns an annotated
relation — is **not implementable** under duckdb-rs 1.4.4's safe API.

This triggers rollback-plan **Scenario A**: scope shrinks to core-library
changes + CLI-only orchestration using the existing scalar
`finetype_validate(value, schema_json)`.

**What still ships (unchanged):**
- ac-01 (TableValidationResult additive fields) — core crate
- ac-02 (constraint_failed canonical tokens) — core crate
- ac-03 (determinism) — core crate
- ac-09 (CLI command shape + staging lifecycle) — CLI
- ac-10 (exit code grid) — CLI
- ac-11 (schema-format input contract) — CLI
- ac-12 (ecommerce_orders e2e) — CLI
- ac-14 (docs + MADR 0064) — docs, adjusted to record Scenario A
- ac-13 (test grid) — pruned: core tests unchanged, DuckDB-table-function
  tests dropped, CLI tests unchanged

**What ships differently (Scenario A):**
- ac-05 (table function) — **DROPPED**. No new DuckDB function registered.
  The scalar `finetype_validate(value, schema_json)` stays as-is.
- ac-06 (annotated-relation projection) — **REWRITTEN** as "CLI
  orchestrates validation via SQL that applies the existing scalar
  across each cell of the staging table, aggregates per row, and
  materialises valid rows + rejects."
- ac-07 (scalar+table coexistence) — **TRIVIALLY SATISFIED** because
  there is no table function; the scalar continues to function.
- ac-08 (malformed-schema clean error) — shifts from "DuckDB table
  function fails at bind" to "CLI's JSON load + per-cell scalar calls
  surface the error uniformly."

**What is reserved for a follow-up card:** the DuckDB-native table
function path. A future card can explore either the C API route or
the Rust-wrapper evolution in a later duckdb-rs release that exposes
catalog access to VTabs.

## Decision reference

This finding is captured in MADR 0064 (to be written as part of ac-14).
The spec update (v1.2) folds the Scenario A adjustments into the
definitive AC list before implementation resumes.

## Artefacts

- `crates/finetype-duckdb/src/spike.rs` — minimal `FineTypeSpike` VTab
- `crates/finetype-duckdb/src/lib.rs` — registers `finetype_spike(n BIGINT)`
  alongside the existing scalars
- Build verification: `cargo build -p finetype_duckdb` — clean, 50.52s
  first build, 3.69s incremental (finding-b code path removed post-probe)
