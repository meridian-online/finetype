# Memo: `finetype validate` requires persistence flags for read-only checks

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a direction
**Tags:** cli, validate, ux

## What happened

```
$ finetype validate eval/datasets/csv/ecommerce_orders.csv eval/datasets/csv/ecommerce_orders.schema.json
error: the following required arguments were not provided:
  --db <DB>
  --table <TABLE>
```

The user wanted a yes/no answer: "is this data valid against this
schema?" The CLI demanded an output database and table name before it
would even start.

## Why the CLI is structured this way

MADR 0064 (`finetype validate` as a DuckDB-native reject pipeline)
defines validate as a **materialisation step**, not a check. The output
is a single DuckDB `.db` file containing two tables: the user's named
table (valid rows only) and a `finetype_reject_errors` sidecar mirroring
DuckDB's native `reject_errors` shape plus four FineType extensions
(`type_confidence`, `expected_type`, `constraint_failed`,
`constraint_value`).

That design is sound for the pipeline intent — analysts get a SQL-native
artefact they can `JOIN`, `UNION ALL`, and query directly. The MADR's
"Good" consequences are real: round-tripping `x-finetype-label` and
`x-finetype-confidence` as `expected_type` / `type_confidence` is
genuinely useful for "classifier wrong vs data bad" triage.

But the CLI surfaces only the materialisation path. There is no
read-only mode.

## The conflation

Two distinct user intents are being served by one verb:

```
| Intent                          | What user wants                                    | Persistence? |
|---------------------------------|----------------------------------------------------|--------------|
| 1. Quick check                  | yes/no/summary — "is this data valid?"             | no           |
| 2. Pipeline materialisation     | valid + reject tables in a .db, query in SQL       | yes          |
```

Today's command only serves intent (2). Intent (1) is the natural
default a new user reaches for, and it doesn't work.

## The engine is already split correctly

The validation engine is `finetype-core::table_validator::validate_table`
(see `crates/finetype-cli/src/main.rs:3901`). It returns a
`TableValidationResult` — pass/fail, per-cell rejects, valid-row indices
— **without touching DuckDB.** The DuckDB shell-out is purely a
materialisation step that happens *after* validation completes
(`main.rs:3911-4014`).

So the underlying engine can already answer the "is my data valid?"
question without `--db` or `--table`. The CLI just doesn't expose that
capability — clap requires both flags upfront.

## Options

**A. Make `--db` and `--table` optional.** When both are omitted, run
validation, print the summary in the format chosen by `-o`, exit with
the existing 0/1/2 codes. Skip the SQL script generation and the
`duckdb` shell-out entirely. When supplied (both, together), keep the
materialisation flow exactly as MADR 0064 specifies. Smallest change,
preserves all existing pipeline behaviour, doesn't add any new flags.

**B. Add a `--check` / `--dry-run` flag.** Same end behaviour as A, but
explicit. Discoverable in `--help`, but adds a flag the user has to
know exists. More ceremony for the more common intent.

**C. Add a separate subcommand.** `finetype check <file> <schema>` for
read-only, `finetype validate ... --db ... --table ...` for the
pipeline. Cleanest semantically but bigger surface and conflicts with
the existing `finetype check` (which validates taxonomy ↔ generator
alignment — different verb, same name).

**D. Auto-derive `--db` and `--table` defaults.** e.g., `<file>.db`
and the file's basename. Lowest friction at the prompt but creates
files on disk silently — surprising in CI scripts and one-off REPL
usage.

Recommendation when we get to deciding: **A**. Match the spirit of MADR
0064 — single engine, single source of truth for pass/fail — but
separate the *output* paths. Persistence becomes opt-in via `--db` +
`--table` (both required together). Read-only check becomes the
default, which is what every new user expects.

The constraint that today says "C must be supplied if D is supplied"
becomes "either both `--db` and `--table` together, or neither." Two
modes, each consistent.

## Implementation sketch (option A)

1. Mark `db: Option<PathBuf>` and `table: Option<String>` in `Commands::Validate` (`main.rs:302-331`).
2. Add a clap `group(args = ["db", "table"], multiple = true, requires_all = ["db", "table"])` so they're mutually-required (clap natively supports this via `requires`).
3. Branch in `cmd_validate_table`:
   - If both `Some`, current path (lines 3909-4031): generate SQL, shell out to `duckdb`, write the .db, exit per reject count.
   - If both `None`, skip lines 3909-4014 entirely; pass `result` directly into the existing `output: OutputFormat` summary printer; exit per reject count.
4. Tests: add `vrp_acNN_check_only_*` covering the new path — read-only smoke, exit codes 0/1, no .db side effects, summary contents.

The current test grid (`crates/finetype-cli/tests/validate_cli.rs`,
15 `vrp_*` tests) all assert with `--db` + `--table` supplied. They
stay green; new tests cover the new mode.

## Documentation impact

- CLI `--help` should describe both modes side-by-side.
- `CHANGELOG.md` notes the new check-only mode (additive — non-breaking).
- The existing MADR 0064 doesn't need superseding; this is an additive
  CLI affordance that uses the same engine. Worth adding a brief MADR
  note ("CLI exposes both check-only and pipeline modes; engine
  unchanged") rather than amending 0064 wholesale.

## Side note: error message could already be more helpful

The current error is clap's stock "required argument missing" prose. If
we don't take option A immediately, an interim improvement: when the
user invokes `finetype validate <file> <schema>` without persistence
flags, suggest the right incantation:

```
error: --db and --table are required
hint: try --db <file.db> --table <name>
hint: read-only validation is not yet supported (see ___)
```

But this is a band-aid — option A is the real fix.

## Not action yet

Observation memo. Promote to a card alongside the schema-export-verbosity
memo (2026-04-27) — both are CLI ergonomics nits surfaced in the same
session, both worth bundling into a single "v0.6.x → v0.7.0 CLI polish"
spec when we want to act.
