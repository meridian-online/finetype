# Validate, DuckDB extension, and the corpus-pass double-read

## Problem

The corpus pass (`scripts/gittables_corpus_pass.py`) reads each input file
**three times** per pass:

1. **profile** reads via `csv::Reader` (Rust)
2. **validate's engine pre-pass** reads via `csv::Reader` (Rust) — runs
   `finetype_core::table_validator::validate_table()` to compute SEMANTIC_TYPE
   rejects and `valid_row_indices`
3. **validate's materialise** reads via DuckDB `read_csv` / `read_parquet`
   (shell-out) — applies typed transforms with `TRY()` for TRANSFORM_FAILED
   detection, writes the user table + `finetype_reject_errors` sidecar

Read 1 and 2 are pure duplication. The corpus pass amplifies the cost
across 1M files. At 16 workers each spawning two `finetype` subprocesses
and one `duckdb` per file, this dominates the 9.3 h Pass A wall clock.

## Where validate-in-DuckDB already lives

`crates/finetype-duckdb/src/lib.rs` registers scalar functions used by the
FineType DuckDB extension (MADR 0064):

- `finetype_validate(value, schema_json) -> 'valid' | error_message`
- `finetype_cast(value, type) -> typed value`
- `finetype_detail(value, schema_json) -> detailed error JSON`
- `finetype_unpack(...)`, `finetype_version()`

`finetype_validate` calls `validate::validate_value()` which uses the same
schema-compilation cache as the Rust pre-pass. So the validation logic is
already DuckDB-callable — the CLI's `finetype validate` just doesn't use
it. The CLI shells out to a bare `duckdb` that doesn't have the extension
loaded, so it falls back to the Rust pre-pass for SEMANTIC_TYPE detection
and uses DuckDB only for the typed CTAS.

## Why "just use the extension" isn't an afternoon's work

Three concrete blockers found while investigating on 2026-05-22:

1. **The community extension is 404 for current DuckDB.**

   ```
   $ duckdb -c "INSTALL finetype FROM community; LOAD finetype;"
   HTTP Error: Failed to download extension "finetype" at URL
   "http://community-extensions.duckdb.org/v1.5.3/osx_arm64/finetype.duckdb_extension.gz" (HTTP 404)
   ```

   CLAUDE.md says "DuckDB community extension (v0.2.0 merged)" but that was
   for an earlier DuckDB version. v1.5.3 (current Homebrew) has no FineType
   extension available. **Republishing for current DuckDB versions is a
   prerequisite** for any path that relies on `INSTALL finetype` in
   end-user CLI commands.

2. **The materialise output is actually consumed.** `scripts/gittables_gate.py:_distinct_rejected_rows_in`
   queries the `.db`'s `finetype_reject_errors` table to count per-column
   distinct rejects. Naively dropping `--db / --table` from the corpus pass
   loses that signal. Validate's JSON summary has `rejects_by_type` totals
   but no per-column breakdown.

3. **Profile isn't in the extension at all.** Only `finetype_validate` and
   helpers. Adding profile would require porting the multi-branch
   classifier + Model2Vec + entity classifier + sibling-context attention
   into the extension binary — a major project. Profile's CSV read stays
   regardless of what we do with validate.

## Two paths worth queuing

### Path A — link `duckdb-rs` into `finetype-cli`, run validate in-process

Replace validate's shell-out-to-`duckdb` with an in-process DuckDB
connection (the `duckdb` Rust crate). Register `finetype_validate`
programmatically against that connection. The whole validate flow —
file read, scalar-function validation, typed CTAS, reject sidecar —
runs in one DuckDB session.

- **Eliminates**: validate's double-read (reads 2 and 3 merge into one
  DuckDB read).
- **Side-steps**: the community-extension 404. No `INSTALL` needed; the
  scalar lives in the same process and is registered at startup.
- **Cost**: ~1-2 days Rust. Adding `duckdb` as a `finetype-cli` runtime
  dep brings static-link weight (DuckDB is ~50MB).
- **Risk**: Touches a load-bearing CLI command. Need full validate test
  suite to pass; the shell-out version stays as a fallback if the
  in-process path errors.

### Path B — add per-column reject counts to validate's JSON summary

Smaller scope. Extend the `-o json` output of `finetype validate` with a
per-column reject row count, so consumers (the corpus pass, MCP) don't
need to query the `.db`'s `finetype_reject_errors` table.

- **Eliminates**: the corpus pass's `_distinct_rejected_rows_in` query.
- **Allows**: dropping `--db / --table` from the corpus pass's validate
  call, which skips DuckDB's materialise CTAS entirely → no more read #3
  for the corpus pass.
- **Cost**: ~½ day Rust + ~1 hour corpus pass change.
- **Does not eliminate**: validate's engine pre-pass (read #2). Path A
  would need to land for that.

Paths A and B are complementary — B is a smaller win that unlocks itself,
A is the deeper restructure that retires the double-read in validate
entirely.

### The 404 — separate work item

Republish `finetype.duckdb_extension` for current DuckDB versions to the
community repo. Two reasons to prioritise even if Paths A/B are pursued:

- Any user-facing CLAUDE.md / DEVELOPMENT.md / docs claim about the
  community extension is silently broken right now.
- Path A side-steps the issue for the CLI, but users invoking the
  scalar function directly from DuckDB (the original MADR 0064 use case)
  still need the published extension.

The republish workflow is documented in `docs/RELEASE.md`'s DuckDB
section (per CLAUDE.md tier-2 references).

## Context

This came up on 2026-05-22 while debugging chunk 3b of the gittables
multi-lens corpus diagnostic (`.orbit/specs/2026-05-20-gittables-multi-lens-diagnostic/`).
Run-1 (v0.6.19) error rate 2.77%; run-3 (v0.6.20 with validate's
parquet engine pre-pass) error rate 3.63% due to macOS posix_spawnp
pressure from extra duckdb spawns. Run-5 (current) reverts the corpus
pass to feed CSV to validate so the spawn count returns to run-1
baseline, with Fix A column-name normalisation handling the original
Binder bucket. The "fused profile+validate" speedup direction got
rerouted to **Path C — batch mode on `finetype profile` (`--files` +
`--out-dir`)** as the immediate ship-today win.

The double-read insight is real and worth eliminating, but A & B are
the right shape for it — not the `scan`-verb fusion or the
"INSTALL finetype" hand-wave I initially suggested.
