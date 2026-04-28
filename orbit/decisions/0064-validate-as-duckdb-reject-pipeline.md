---
status: accepted
date-created: 2026-04-24
date-modified: 2026-04-28
---
# 0064. `finetype validate` as a DuckDB-native reject pipeline

## Context and Problem Statement

v1.1 of spec `2026-04-22-duckdb-extension-ergonomics` proposed a new DuckDB
table function `finetype_validate_table(table, schema)` that would read rows
from an existing DuckDB table and emit valid rows and reject records in the
same pass. This refined the older CSV-sidecar flow (`.valid.csv`,
`.invalid.csv`, `.errors.jsonl`) that MADR 0031 / 0032 shipped — the sidecar
format was awkward to consume from SQL and duplicated DuckDB's native
`reject_errors` shape.

The ac-04 gate spike (`spike.rs`, registered as `finetype_spike`) set out to
prove `vtab` feasibility under `duckdb-rs` 1.4.4 in the `loadable-extension`
safe API. Finding (a): vtab feature is available. Finding (b): scalar + table
functions coexist in a single registration. **Finding (c) blocked the v1.1
shape:** `BindInfo` in duckdb-rs 1.4.4 exposes no `Connection`, catalog
lookup, or row-iteration primitive — so a table function registered via the
safe API cannot read from an existing DuckDB table by name. The feature
needed the unsafe low-level FFI or a newer duckdb-rs with an extended
`BindInfo`; neither was a stop-writing-production-code commitment.

Full spike findings: `orbit/specs/2026-04-22-duckdb-extension-ergonomics/spike-duckdb-rs.md`.

## Considered Options

- **A. Ship CLI-only (no new DuckDB function).** `finetype validate` keeps
  a single production validation path in `finetype-core::table_validator`,
  shells out to the `duckdb` CLI to stage the input and write both the
  user table and a sidecar `finetype_reject_errors` table inside one `.db`
  file. No new DuckDB registration; the existing scalar
  `finetype_validate(value, schema_json)` stays untouched.
- **B. Drop to unsafe low-level FFI for a table function.** Implement
  `finetype_validate_table` via raw duckdb-rs FFI calls to open a
  sub-connection inside `bind()`, reach the catalog, and iterate rows.
  High implementation and maintenance cost; breaks the safe-API envelope
  the rest of `finetype-duckdb` lives in.
- **C. Wait for duckdb-rs to extend `BindInfo`.** Park the spec until the
  upstream crate exposes `Connection` in `BindInfo`. Indefinite blocking;
  we'd lose the authored-time `x-finetype-*` schema round-trip that's
  the whole user value of this work.

## Decision Outcome

Chosen option: **A**, because it preserves the spec's user value — the
authored-time confidence signal, the 13-column reject ontology mirroring
DuckDB's `reject_errors`, the SQL-in-situ `.db` artefact — without
committing to unsafe FFI or an indefinite wait for an upstream change.

The v1.1 rollback_plan anticipated this ratification explicitly as
"Scenario A": spec rewritten to v1.2, the duckdb table function dropped,
the CLI becomes the sole caller of `finetype-core::table_validator`, and
the existing scalar `finetype_validate` is preserved. `spike.rs` is
retained as living compile-time evidence of findings (a), (b), (c).

This MADR refines and supersedes MADR 0031 / 0032 on the CSV-sidecar
validation flow — the sidecar CSVs are no longer produced by
`finetype validate`. The destination is a single DuckDB `.db` file with
two tables: the user-named table of valid rows, and
`finetype_reject_errors`.

### Consequences

- **Good**, because validation is a pure function over
  `(schema, rows) → TableValidationResult`, kept in finetype-core as the
  single engine; the CLI is the only caller in the default flow.
- **Good**, because the reject sidecar shares DuckDB's base columns
  (`scan_id, file_id, line, column_idx, column_name, error_type,
  csv_line, byte_position, error_message`) plus FineType's four
  extensions (`type_confidence, expected_type, constraint_failed,
  constraint_value`) — a UNION against native `reject_errors` only
  requires explicit NULL-pad projection.
- **Good**, because authored-time `x-finetype-label` and
  `x-finetype-confidence` round-trip as `expected_type` and
  `type_confidence` in the sidecar, surfacing the "classifier wrong"
  vs "data bad" distinction directly to analysts in SQL.
- **Good**, because TEMPORARY staging tables auto-drop when the DuckDB
  session ends — RAII-equivalent cleanup on success AND failure without
  a manual unwind path.
- **Good**, because `finetype_validate(value, schema_json)` remains
  unchanged for ad-hoc SQL use; no breaking change to the existing
  DuckDB extension surface.
- **Neutral**, because the CLI shells out to the `duckdb` binary rather
  than linking a Rust duckdb client. The workspace already pins
  `loadable-extension` features that conflict with a `bundled` client;
  shelling out sidesteps the feature-unification issue and keeps the
  runtime dep the same binary analysts already have on PATH.
- **Bad**, because CLI users now need `duckdb` on PATH (previously the
  sidecar flow had no runtime dep). Documented in the CLI help and
  release notes.
- **Bad**, because we cannot offer a pure-SQL entrypoint for table
  validation until duckdb-rs's safe API matures. Analysts who want the
  reject pipeline must go through the CLI, not `SELECT ... FROM
  finetype_validate_table(...)`.

## Implementation

Spec: `orbit/specs/2026-04-22-duckdb-extension-ergonomics/spec.yaml` (v1.2).

- finetype-core: `TableValidationResult` gains `valid_row_indices: Vec<usize>`
  and `rejects: Vec<RejectRecord>` (additive). `RejectRecord` has 9 fields
  (row_index, column_index, column_name, error_message, constraint_failed,
  constraint_value, error_type, type_confidence, expected_type).
- finetype-core: validator loop now iterates every jsonschema error per
  cell and maps `ValidationErrorKind` to canonical tokens
  (pattern, min_length, max_length, enum, type, required). Explicit sort
  by (row_index, column_index) for determinism.
- finetype-cli: `finetype validate <input> <schema.json> --db <output.db>
  --table <name>` replaces the old CSV-sidecar path. Flags: `--append`,
  `--lenient`. Exit codes: 0 (no rejects), 1 (rejects without --lenient),
  2 (error). Staging table is TEMPORARY; input read via DuckDB's
  `read_csv` / `read_parquet`.
- finetype-duckdb: `spike.rs` retained as living evidence; no new
  production function registered. The scalar `finetype_validate(value,
  schema_json)` is unchanged.

Test grid: 15 `vrp_acNN_*` functions across two crates (core 7 + CLI 8),
all passing.
