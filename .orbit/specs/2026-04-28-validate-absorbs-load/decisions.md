# Decision pack — `validate` absorbs `load` (card 0005)

**Card:** `.orbit/cards/0005-schema-driven-data-validation.yaml`
**Memo:** `orbit/memos/2026-04-27-load-folds-into-validate.md`
**Inherits spec:** `.orbit/specs/2026-04-22-duckdb-extension-ergonomics/spec.yaml` (v1.2)
**Refines:** MADR 0064, 0031, 0032
**Target release:** v0.6.19 (second half — pipeline-reshape)

## Context for this pack

The first half of v0.6.19 (PR #51, spec
`2026-04-27-cli-visibility-cleanup`) shipped:

- `--db`/`--table` made optional-but-mutually-required on `validate`
  (clap `requires` cross-references at `main.rs:307–319`).
- `validate` defaults to **check-only mode** when neither flag is
  supplied — `cmd_validate_table` skips DuckDB entirely
  (`main.rs:3827–3962`, the `if let (Some(db_path), Some(table_name))`
  guard around the materialise block).
- `--model` removed; `FINETYPE_MODEL` env var is the single override
  knob.
- `load` stayed public this release (constraint in
  `2026-04-27-cli-visibility-cleanup/spec.yaml:21`); its consolidation
  was deferred to this card.

What's left to design here is the **fold of `load` into `validate`** —
making `validate --db --table` write a *typed* user table instead of a
VARCHAR pass-through, by lifting the per-column transform projection
that `cmd_load` already builds (`main.rs:3061–3328`,
`build_load_expr` at 3336–3353) into the `validate` write path.

## Concrete code shape today

The validate write path emits this CTAS at `main.rs:3884`:

```rust
"CREATE TABLE {} AS SELECT * EXCLUDE(__row_idx) FROM {} {};"
```

The staging is `read_csv(..., all_varchar=true)` (`main.rs:3856`), so
every user-table column today is VARCHAR. `cmd_load`
(`main.rs:3061–3328`) builds the per-column projection from
`taxonomy.ddl_info(label)` (`taxonomy.rs:668–685`,
returning `duckdb_type` + `transform`) via `build_load_expr`
(`main.rs:3336–3353`):

```rust
fn build_load_expr(original_name, duckdb_type, transform) -> String {
    let col_ref = format_column_name(original_name);    // "col"
    if duckdb_type == "VARCHAR" { col_ref }              // bare passthrough
    else if let Some(tf) = transform { tf.replace("{col}", &col_ref) ... }
    else { format!("CAST({} AS {}) AS {}", col_ref, duckdb_type, col_ref) }
}
```

`cmd_load` resolves the **label per column** by re-profiling the file
(`column_classifier.classify_column_with_header(...)` at
`main.rs:3179–3183`). Folded `validate` doesn't need to re-profile —
the schema already carries `x-finetype-label` per column,
read by `SchemaExtensions::extract` at `main.rs:3606–3621`.

## How `cmd_load` handles a "no transform" column today

`build_load_expr` (`main.rs:3336–3353`) renders the projection as:

- VARCHAR / generic: bare `"col"` (line 3344)
- non-VARCHAR + transform: `tf.replace("{col}", ...)` aliased
  `AS "col"`
- non-VARCHAR + no transform: `CAST("col" AS T) AS "col"`

So **passthrough is "bare quoted identifier"**, not a comment marker.
The comment marker (`-- {label}`) is only written *alongside* the
expression in the printed SQL (`main.rs:3305`); it is not part of the
SQL DuckDB sees. The memo's claim
(`load-folds-into-validate.md:144–145`) that `cmd_load` uses a comment
marker for no-transform columns is slightly misleading — it does, but
only as decoration in the rendered output, not as a SQL no-op.

## `finetype_reject_errors` ontology — what's available for transform failures

From `2026-04-22-duckdb-extension-ergonomics/spec.yaml` (RejectEntry
ontology, lines 348–395) and `REJECT_SIDECAR_DDL` in
`main.rs:3713–3728`:

- `error_type VARCHAR` — currently only `'SEMANTIC_TYPE'` is emitted
  (constants at `main.rs:3900`).
- `constraint_failed VARCHAR` — `'pattern' | 'min_length' |
  'max_length' | 'enum' | 'type' | 'required' | 'other'` (spec ac-02).
- `constraint_value`, `expected_type`, `type_confidence`,
  `error_message`, `csv_line` (NULL today), `byte_position` (NULL today).

A transform failure does not naturally map onto any current
`constraint_failed` token. The memo proposes a new token — it sits in
the same ontology slot as `pattern`/`enum`/etc.

---

## Decision 1 — Surfacing transform failures

### Context

A row that passed schema validation can still fail its DuckDB
transform (e.g., `2024-02-30` matches `^\d{4}-\d{2}-\d{2}$` but
`strptime(..., '%Y-%m-%d')` fails). Today this is a
non-issue because `validate`'s CTAS writes VARCHAR and `cmd_load`'s
CTAS is run by the user separately — they just see DuckDB's error.
After folding, the transform runs *inside* the validate session, and a
silent NULL or session crash both swallow the diagnostic the analyst
needs. The memo (`load-folds-into-validate.md:98–127`) recommends
TRY-wrap each transform and emit a `TRANSFORM_FAILED` reject record.

### Options

- **A. TRY-wrap each transform in the projection; emit a reject row
  when the wrapped value is NULL but the staging value was not.** New
  token `constraint_failed = 'transform'` (or `'transform_failed'`)
  added to the existing 7-value enum
  (`pattern|min_length|max_length|enum|type|required|other`); reject
  rows reuse `error_type = 'SEMANTIC_TYPE'` or get a new
  `error_type = 'TRANSFORM_FAILED'` discriminator. The transform row
  is excluded from the user table.
- **B. Run validation a second time post-transform.** Materialise
  valid rows into a temp table with transforms applied, then re-run
  the validation engine reading the typed cells. Catches DuckDB-level
  type errors as a second pass.
- **C. Let DuckDB crash on transform failure (status quo for
  `cmd_load`).** Run the CTAS without TRY wrappers; if any row fails
  any transform, the entire validate command fails with a DuckDB
  error. No reject row is emitted for the failing cell.

### Trade-offs

- **A** preserves the single-CTAS shape (`load-folds-into-validate.md:46`,
  `main.rs:3915–3930`'s one-transaction script) and reuses the
  existing `finetype_reject_errors` 13-column shape
  (`spec.yaml:347–395`) with one ontology extension. The cost is one
  new constraint token + a small projection-builder change to detect
  "staging non-NULL and transform NULL". Surfaces the diagnostic the
  analyst needs (memo section "What about transform failures?",
  lines 100–127). Constraint-token enum stays small (8 values vs 7).
- **B** gives the strongest guarantees — every cell in the user table
  has been validated as both schema-conforming AND transform-conforming
  — but runs the validator twice and breaks the
  "single validation engine" principle from MADR 0064 consequence #1
  ("validation is a pure function over (schema, rows) →
  TableValidationResult"). It also requires re-implementing
  validation against typed cells (today's validator takes
  `Vec<Vec<Option<String>>>` per `validate_table` at
  `main.rs:3819` and `mcp/tools/validate.rs:86`). Memo explicitly
  rejects this: "Slow; not necessary"
  (`load-folds-into-validate.md:121`).
- **C** matches today's `cmd_load` behaviour (`main.rs:3061–3328`
  emits no error handling around the CTAS), but inside `validate` it
  conflicts with the contract that `validate` exits 0 / 1 / 2 cleanly
  and writes a reject sidecar (MADR 0064 consequences). A DuckDB
  session crash mid-transaction means partial state and an exit-code
  collision (where does it go — exit 1 reject, or exit 2 error?). The
  whole point of folding is making validate the honest partition; a
  silent crash is the dishonest path.

### Recommendation

**A — TRY-wrap with a new `constraint_failed = 'transform'` token,
keeping `error_type = 'SEMANTIC_TYPE'` for ontology stability.**
Emit one reject row per failing cell mirroring the existing
`SEMANTIC_TYPE` shape (`main.rs:3900`); populate
`constraint_value` with the raw staging value (or the transform
expression as a debug string). The new token is the smallest
extension to the 7-value enum and slots into the existing test grid
(`vrp_ac02_constraint_grid`). Excluded-from-user-table semantics fall
out naturally if the transform projection is built with
`TRY(transform) AS col` in the CTAS and the row is moved from
`valid_row_indices` to a transform-fail set in the CLI before INSERT.

Open detail for the spec: whether `error_type` stays `SEMANTIC_TYPE`
or splits into `'SEMANTIC_TYPE'` / `'TRANSFORM_FAILED'`. The memo
recommends a new error_type (`load-folds-into-validate.md:122–125`),
but the spec ontology
(`spec.yaml:373–374`) describes `error_type` as a discriminator for
UNIONing with DuckDB's native parse rejects — adding a third value
that's still a FineType-side reject is consistent with that role.
Recommend: split. `error_type = 'TRANSFORM_FAILED'` makes the SQL
filter `WHERE error_type = 'TRANSFORM_FAILED'` trivial and keeps
`SEMANTIC_TYPE` precisely meaning "schema rejection."

---

## Decision 2 — Code home for the projection builder

### Context

`build_load_expr` at `main.rs:3336–3353` is the per-column projection
generator today. The folded validate path needs the same logic. The
memo claims it should "lift to a shared helper, parameterise on
whether to TRY-wrap" (`load-folds-into-validate.md:243–245`). The
question is *where* the helper lives.

### Options

- **A. Lift `build_load_expr` to a new free function in
  `finetype-cli/src/main.rs` (or a sibling module under
  `crates/finetype-cli/src/`), call from both `cmd_validate_table` and
  `cmd_load` until `cmd_load` is removed.** No new crate dependency.
- **B. Lift to `finetype-core` (e.g., `transform_projection.rs`
  module).** Reachable from MCP (`crates/finetype-mcp/src/tools/`)
  and DuckDB extension (`crates/finetype-duckdb/src/`) as well as CLI.
- **C. Don't lift — remove `cmd_load` first (in this same PR), then
  inline the projection builder inside `cmd_validate_table` only.**
  No shared helper; one code site.

### Trade-offs

- **A** gives the cleanest minimum-change shape: lift, dedupe, ship.
  The helper is ~18 LOC today and only depends on
  `taxonomy::DdlInfo` + `format_column_name` (`main.rs:3397–3399`,
  CLI-local). Both call sites compile against it; tests for either
  path exercise it. Cost: temporary duplication if `cmd_load` removal
  slips (Decision 4).
- **B** matches the MADR 0064 principle of "single source of pass/fail
  + reject detail" by extension to "single source of typed projection."
  However, `crates/finetype-cli/src/main.rs:3397` shows
  `format_column_name` and `sql_ident` (`main.rs:3640–3643`) live in
  CLI code, not core. Moving the projection builder to core would
  require also moving (or duplicating) those helpers; the SQL-quoting
  surface area then accretes in core and starts shaping
  `finetype-core`'s public API for a CLI concern. Today
  `finetype-core::table_validator` is engine-only (no SQL string
  emission); pushing SQL there crosses an existing layer boundary.
  MCP doesn't currently emit SQL (`mcp/tools/validate.rs:86` only
  calls `validate_table`); DuckDB extension does its own SQL through
  the loadable-extension API, not through string templates.
- **C** is the absolute-minimum-LOC path: `cmd_load` deletion happens
  in the same PR, the projection builder lands once inside
  `cmd_validate_table`, no shared-helper overhead. But it conflates
  two concerns (the fold + the deletion) and prevents incremental
  shipping if Decision 4 lands the deletion in a follow-up release.

### Recommendation

**A — lift to a free function in `finetype-cli/src/main.rs`,
parameterised on `try_wrap: bool`.** Keeps SQL-emission code in the
CLI crate where it already lives (sibling to `sql_ident`,
`sql_quote`, `format_column_name`). Both `cmd_validate_table`
and `cmd_load` call it; when `cmd_load` is removed (Decision 4),
the helper survives and the file gets ~270 LOC simpler.

Future migration: if a non-CLI consumer (DuckDB native table function
in some post-2026 iteration, or an MCP tool that materialises) needs
the same projection logic, *then* lift to `finetype-core`. Today
that's speculative — no consumer outside the CLI emits typed SQL.

---

## Decision 3 — `--no-transform` escape hatch

### Context

The memo identifies an edge case: a user wants raw VARCHAR output
from `validate` (e.g., feeding a downstream tool that expects strings,
or debugging the staging shape). Two answers
(`load-folds-into-validate.md:130–141`):

- Add a `--no-transform` flag, falling back to today's
  `* EXCLUDE(__row_idx)` projection (`main.rs:3884`).
- Refuse the flag; users `SELECT CAST(col AS VARCHAR) FROM orders`
  after the fact.

### Options

- **A. Ship `--no-transform` now.** New clap flag on `validate` (only
  meaningful when `--db`/`--table` are supplied). When set,
  `cmd_validate_table` keeps today's `SELECT * EXCLUDE(__row_idx)`
  projection (`main.rs:3884`). When unset, applies the transform
  projection.
- **B. Refuse the flag; document the SQL-after-the-fact pattern.**
  Validate always emits the typed projection when materialising. Users
  cast post-hoc.
- **C. Ship `--no-transform`, but mark it `#[command(hide = true)]`
  (a v0.6.19 pattern from PR #51) and document it as a debug
  affordance.** Available for diagnostic use, doesn't bloat
  `--help`.

### Trade-offs

- **A** is the user-friendly default but adds a long-lived public flag
  for a rare case. The memo is explicit: "Don't add the flag. SQL is
  the answer when SQL is the context"
  (`load-folds-into-validate.md:140–141`). Once a flag ships public
  it's hard to remove (cf the v0.6.19 hard-removal posture in
  `2026-04-27-cli-visibility-cleanup/spec.yaml:16` — every public
  surface change costs a deprecation cycle to undo).
- **B** matches the memo's recommendation and the Meridian "spark joy
  for analysts" pillar via fewer flags. Costs: a user encountering a
  transform failure under TRY-wrap mode (Decision 1) can't easily
  reach VARCHAR output to debug their schema; their workaround is
  re-running with the schema's `x-finetype-label` removed from the
  failing column, which is fiddly.
- **C** is a diagnostic carve-out — present for debugging, absent from
  `--help`. Mirrors the existing pattern for `--sharp-only` before
  PR #51 (`main.rs:367` historically had `hide = true`). Cost: another
  hidden flag accretes (the visibility-cleanup spec just *removed*
  several of these).

### Recommendation

**B — refuse the flag.** SQL after the fact is the right escape hatch.
The memo's reasoning holds: when the user is already in a DuckDB `.db`
context with the schema's transform information available
(`x-finetype-label` round-trips into `expected_type` via
`SchemaExtensions::extract` at `main.rs:3606–3621` and
`finetype_reject_errors.expected_type`), `SELECT CAST(col AS VARCHAR)`
is one line and locally inspectable.

If diagnostic users surface, prefer documenting the SQL pattern in
the README's `validate` section (already noted as a follow-up doc
refresh in `2026-04-27-cli-visibility-cleanup/spec.yaml:23`) over
adding a flag.

---

## Decision 4 — Migration plan for `cmd_load`

### Context

Once `validate --db --table` writes typed output, `cmd_load` produces
a strict subset of `validate`'s output (no validation, no reject
sidecar, only the typed CTAS — and prints SQL to stdout rather than
materialising). Three migration shapes are available; the
visibility-cleanup spec already shipped one of them (hide-via-clap)
for `check`, `generate`, `train`, `train-multi-branch`, `eval`,
`infer-batch` (see `2026-04-27-cli-visibility-cleanup/progress.md:10`).

### Options

- **A. Hide `cmd_load` via `#[command(hide = true)]` in v0.6.19, keep
  the implementation, deprecate-and-warn at runtime.** Same mechanism
  as the 6 hidden subcommands (`spec.yaml:24–25`). `cmd_load` keeps
  working for any caller pinning the old shape; emits a stderr warning
  pointing at `validate --db --table`.
- **B. Remove `cmd_load` outright in v0.6.19.** Variant deleted from
  the `Commands` enum (`main.rs:225–265`), dispatch arm gone
  (`main.rs:631`), `cmd_load` function deleted (`main.rs:3061–3328`,
  ~270 LOC), `build_load_expr` lives only because the lifted helper
  in Decision 2 still uses it. `finetype load file.csv` now errors via
  clap's unknown-subcommand handler (matches v0.6.19's hard-removal
  posture for `eval-gittables`, `--model`, `--sharp-only`).
- **C. Keep `cmd_load` public, no warning, ship in v0.6.19; remove in
  a later release.** One release of overlap. Both verbs work; no
  user-visible breakage.

### Trade-offs

- **A** matches the **internal-keep / public-hide** pattern documented
  in `CLAUDE.md`'s new "Public vs internal CLI surface" section
  (CLAUDE.md, "Public vs internal CLI surface" section). But `load`
  has no internal-caller justification: it's a thin, user-facing
  wrapper. Hiding it preserves a code path that would otherwise be
  dead (no `make ci` target uses it; no script depends on it — a grep
  of the v0.6.19 visibility-cleanup migration list at
  `2026-04-27-cli-visibility-cleanup/spec.yaml:18` does not mention
  `cmd_load`). Deprecate-and-warn is a runtime behaviour the spec
  hasn't specified before; new precedent.
- **B** matches the **hard-removal posture** the visibility-cleanup
  spec ratifies (`spec.yaml:16`: "Hard removal posture — no
  deprecation cycle. Removed flags/subcommands error immediately via
  clap's unknown-arg/unknown-subcommand handling. Scripts pinning old
  surfaces fail loudly rather than drift silently"). Removes ~270 LOC
  (`cmd_load` function) and the Load variant + dispatch arm. A user
  scripting `finetype load file.csv` gets a clap error pointing at
  `--help`, where `validate` is one row up. Recovery is
  one-line. Memo explicitly proposes this:
  "`cmd_load` deprecated → removed (no replacement; `validate`
  covers it)" (`load-folds-into-validate.md:253–256`).
- **C** preserves user habit briefly but contradicts the v0.6.19
  posture (the same release that just removed `--model`,
  `--sharp-only`, and `eval-gittables` outright). Two CTAS-emitting
  verbs is exactly the redundancy this card is removing. The "two
  verbs" pipeline shape (`load-folds-into-validate.md:55–73`) is
  the user-facing payoff.

### Recommendation

**B — remove `cmd_load` outright in v0.6.19.** The visibility-cleanup
spec just established the hard-removal posture as the v0.6.19 norm.
`cmd_load` has no internal callers (`make ci` doesn't use it, no
training script does, no test does outside golden tests for `load`
itself). Migration is a one-liner: `finetype load file.csv -t orders`
becomes `finetype validate file.csv schema.json --db out.db --table
orders` (with `schema.json` from `finetype schema file.csv` —
two commands instead of one for the no-schema case, but the analyst
gets a *durable schema artefact* as a side effect, which is the
correct shape per memo's "What about 'I want types but I trust my
data'?" section, lines 175–197).

Update golden tests under `crates/finetype-cli/tests/cli_golden.rs` —
the `load` test fixtures get replaced with `validate --db --table`
fixtures (or removed if `validate_cli.rs` already covers them at
`crates/finetype-cli/tests/validate_cli.rs:110–447`).

Out of scope for this card / decision: an `--infer-schema` flag on
`validate` that profiles internally to produce the schema. The memo
addresses this (`load-folds-into-validate.md:185–197`): "ship the
chain first."

---

## Decision 5 — Schema lacks `x-finetype-label` on a column

### Context

`SchemaExtensions::extract` at `main.rs:3606–3621` reads
`x-finetype-label` per column and returns `(Option<String>,
Option<f64>)` per column name. When absent, both fields are `None`,
and the reject sidecar populates `expected_type` and `type_confidence`
as NULL (memo: "graceful degradation,"
`load-folds-into-validate.md:147–151`; spec ac-11 graceful degradation
clause at `spec.yaml:259`).

In folded validate, this column also needs a *projection rule*: with
no label, there's no taxonomy entry, so no `transform` and no
`duckdb_type` to cast to.

### Options

- **A. Bare quoted identifier (VARCHAR pass-through).** Same shape as
  the VARCHAR branch of `build_load_expr` (`main.rs:3344`). Column
  remains VARCHAR in the user table, alongside transformed columns.
- **B. Drop the column from the user table.** No label = no opinion =
  no output. User explicitly opts in by adding `x-finetype-label` to
  the schema.
- **C. Refuse the validate run with exit 2, "schema is missing
  x-finetype-label for column X."** Forces explicit schemas.

### Trade-offs

- **A** matches today's behaviour (`SchemaExtensions::get` returning
  `(None, None)` and `validate_table` at `main.rs:3819` proceeding to
  validate the column against whatever standard JSON Schema constraints
  exist). The memo confirms this:
  "columns without an `x-finetype-label` would simply pass through
  VARCHAR. Same graceful degradation"
  (`load-folds-into-validate.md:148–151`). Cost: mixed-type user
  tables (some VARCHAR, some typed) which is exactly what user-authored
  schemas with hand-edited columns should produce.
- **B** is destructive — analysts whose hand-authored schemas omit
  `x-finetype-label` for a column get a *missing column* in the user
  table. A debugging trap.
- **C** is the strict shape but conflicts with the existing graceful
  degradation contract documented in `spec.yaml:259` ("Schemas without
  these extensions validate correctly but produce NULL in the
  corresponding reject columns") and tested in
  `validate_cli.rs:411` (`test_vrp_ac11_null_on_absence`).

### Recommendation

**A — bare quoted identifier, VARCHAR pass-through.** Preserves the
existing graceful-degradation contract from MADR 0064 / ac-11 and
matches `cmd_load`'s VARCHAR branch (`main.rs:3344`). This is also
the natural fallthrough of `build_load_expr` when
`taxonomy.ddl_info(label)` returns `None` (label not in taxonomy):
the projection emits the bare identifier
(`main.rs:3192`'s `("VARCHAR".to_string(), None, String::new())`
branch).

Constraint to lift into the spec: ensure existing
`vrp_ac11_null_on_absence` continues to assert NULL
`expected_type`/`type_confidence` in the reject sidecar AND that the
user table contains the column as VARCHAR.

---

## Decision 6 — MCP `validate` tool: gain materialise mode or stay engine-only?

### Context

`crates/finetype-mcp/src/tools/validate.rs:60–186` runs
`finetype-core::table_validator::validate_table` and returns a JSON
summary + markdown. It does *not* write to DuckDB; it has no `db` /
`table` parameter (`ValidateRequest` at lines 9–22 takes only `path`,
`data`, `schema`). It mirrors the CLI's check-only mode.

### Options

- **A. Keep MCP validate engine-only — no materialise mode.** MCP
  agents that want a typed `.db` shell out to the CLI binary
  themselves (or invoke the existing `infer` / `profile` tools). MCP's
  `validate` returns the same JSON it returns today; MCP's `ddl` tool
  remains the typed-output path
  (`crates/finetype-mcp/src/tools/ddl.rs`).
- **B. Add `db`/`table` to MCP's `ValidateRequest`; MCP gains the
  materialise + transforms behaviour.** Same shape as the CLI:
  optional-but-mutually-required, transforms applied when supplied.
  Requires MCP to shell out to `duckdb` (matching CLI's
  `main.rs:3936`) — agent's process must have `duckdb` on PATH.
- **C. Add `db`/`table` to MCP, but MCP runs the typed CTAS via
  duckdb-rs (no shell-out).** Avoids the PATH dependency MCP
  agents would otherwise need to satisfy. Cost: introduces a duckdb-rs
  client dependency on `finetype-mcp` that today has none, and the
  workspace's existing `loadable-extension` feature pin
  (MADR 0064 consequence: "the workspace already pins
  `loadable-extension` features that conflict with a `bundled`
  client") would need re-evaluation.

### Trade-offs

- **A** is the surgical option. MCP's `validate` is symmetric with the
  CLI's check-only mode (which is the new v0.6.19 default). The
  visibility-cleanup spec already pinned MCP as out-of-scope
  for v0.6.19 (`spec.yaml:19`: "MCP server is out of scope. MCP's
  `schema` tool stays verbose for v0.6.19"). Following that posture
  means MCP changes ride a separate spec ("MCP surface audit"
  follow-up at `cli-visibility-cleanup/spec.yaml:372`). Cost: an MCP
  agent driving the import flow needs two tool calls (validate, then
  shell out for materialise) or has to use the CLI binary directly.
- **B** is symmetric with the CLI but introduces a runtime PATH
  dependency on MCP that wasn't there before, and a 13-column reject
  ontology that the MCP wire format would need to surface in the
  JSON response. Doable, but a significant scope add.
- **C** unifies the runtime engine but expands the dependency graph;
  the explicit feature-pin issue from MADR 0064 means it isn't a
  drop-in.

### Recommendation

**A — keep MCP engine-only for v0.6.19; defer materialise mode to
the post-pipeline-reshape "MCP surface audit" follow-up
(`cli-visibility-cleanup/spec.yaml:372`).** The card's primary scope
is folding `cmd_load` into `cmd_validate_table` in the CLI; expanding
MCP simultaneously triples the diff and entangles two release themes.
MCP today already exposes the right primitives for an agent to drive
the typed-output flow: `validate` (engine), `ddl` (typed-output SQL),
`schema` (export). An agent that needs the typed `.db` calls those
in sequence; symmetric materialise behaviour can land in a follow-up
once the CLI shape is settled.

Constraint for the spec: the MCP `validate` tool's `ValidateRequest`
shape (`mcp/tools/validate.rs:9–22`) does NOT change in this card; no
new fields, no behavioural change. Tests under
`crates/finetype-mcp/` continue to pass unchanged.

---

## Summary of recommendations

```
| # | Decision                                  | Recommendation                                              |
|---|-------------------------------------------|-------------------------------------------------------------|
| 1 | Transform-failure surfacing               | TRY-wrap; new constraint_failed='transform' token; split    |
|   |                                           | error_type into 'SEMANTIC_TYPE' / 'TRANSFORM_FAILED'         |
| 2 | Projection-builder code home              | Lift to free function in finetype-cli/src/main.rs           |
| 3 | --no-transform escape hatch               | Refuse the flag; users SELECT CAST post-hoc                 |
| 4 | cmd_load migration                        | Remove outright in v0.6.19 (matches hard-removal posture)   |
| 5 | Schema missing x-finetype-label           | Bare quoted identifier (VARCHAR pass-through)               |
| 6 | MCP validate tool materialise mode        | Keep engine-only; defer to MCP surface audit follow-up      |
```

## Open questions for the spec author (out of scope for this design pack)

- **Transform-time NULL semantics.** Today the CSV reader at
  `main.rs:3804–3812` normalises `""`, `"NULL"`, `"null"` to `None`.
  In a TRY-wrapped projection, distinguishing "staging NULL → typed
  NULL (legitimate)" from "staging non-NULL → typed NULL
  (transform failed)" needs care: a NULL-in / NULL-out pair must NOT
  emit a `TRANSFORM_FAILED` reject row.
- **The `valid_row_indices` set after transform failures.** Decision 1
  removes failed-transform rows from the user table. Implementation
  question: does the validate engine return `valid_row_indices` and
  the CLI splits them into "passed transforms" / "failed transforms"
  post-CTAS? Or does the projection write to a temp table first, the
  CLI inspects which rows transformed cleanly, and *then* the CLI
  inserts into the user table? The first option matches the
  single-CTAS shape; the second is more honest about
  insert-conditional-on-transform-success but doubles the SQL
  emitted.
- **Reject-sidecar `constraint_value` for transform failures.** Should
  it carry the transform expression literally
  (`strptime("col", '%Y-%m-%d')::DATE`), or the failing input value,
  or both? Memo doesn't pin this. Suggest: the failing input value
  (matches existing `pattern`-token semantics where
  `constraint_value` is the regex, and `error_message` carries the
  failing value). For `transform`, `constraint_value` could carry the
  transform expression (mirrors regex-as-constraint) and
  `error_message` carries the staging cell value.
