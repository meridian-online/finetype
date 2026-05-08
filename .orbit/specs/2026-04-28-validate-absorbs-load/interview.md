# Design: `validate` absorbs `load`

**Date:** 2026-04-28
**Interviewer:** Nightingale (lead, with author Hugh)
**Card:** `.orbit/cards/0005-schema-driven-data-validation.yaml`
**Rally:** `.orbit/specs/2026-04-28-v0619-cli-consolidation-rally/`
**Sibling cards:** 0003 (profile -o json-schema), 0006 (schema verb fold)
**Target release:** v0.6.19 (second half of CLI surface consolidation)

---

## Context

**Card:** *Schema-driven data validation* — 6 scenarios, goal: "Validate
is the one verb for fit-for-use data — pass/fail engine + typed output
table + reject sidecar — with the same engine reachable from DuckDB SQL."

**Prior specs:**
- `2026-03-12-validate-command/` — original validate command shape
- `2026-04-22-duckdb-extension-ergonomics/spec.yaml` — MADR 0064
  reject pipeline (shipped, PR #46)
- `2026-04-27-cli-visibility-cleanup/` — first half of v0.6.19 (shipped,
  PR #51): check-only mode, optional-but-mutually-required `--db`/
  `--table`, `--model` flag removal

**Gap:** `validate --db --table` writes a VARCHAR-only user table today.
`load` re-reads the file, builds a per-column transform projection,
and emits a separate typed CTAS. The two operations should be one. The
memo `2026-04-27-load-folds-into-validate.md` lays out the architecture;
this design pack picked the implementation choices.

**Source memos:**
- `orbit/memos/2026-04-27-load-folds-into-validate.md` (the architectural
  memo this card crystallises)
- `orbit/memos/2026-04-27-validate-required-flags.md` (already shipped,
  PR #51)

---

## Q&A

### Q1: How do transform failures surface?

**Q:** A row that passes schema validation can still fail its DuckDB
transform (`2024-02-30` matches `^\d{4}-\d{2}-\d{2}$` but `strptime`
fails). After folding, the transform runs inside the validate session.
Silent NULL? Session crash? Reject record?

**A:** TRY-wrap each transform in the projection. When the wrapped
value is NULL but the staging value was non-NULL, emit a reject row
with a new `error_type = 'TRANSFORM_FAILED'` discriminator and a new
`constraint_failed = 'transform'` token. The row is excluded from the
user table. Both ontology slots get one new value:

- `error_type` enum extends from `{'SEMANTIC_TYPE'}` to
  `{'SEMANTIC_TYPE', 'TRANSFORM_FAILED'}`
- `constraint_failed` enum extends from
  `{'pattern','min_length','max_length','enum','type','required','other'}`
  to that set ∪ `{'transform'}`

Splitting `error_type` (rather than overloading `'SEMANTIC_TYPE'`)
makes the SQL filter `WHERE error_type = 'TRANSFORM_FAILED'` trivial
and keeps `'SEMANTIC_TYPE'` precisely meaning "schema rejection."

### Q2: Where does the projection-builder live?

**Q:** `build_load_expr` at `main.rs:3336-3353` is `cmd_load`'s today.
Lift to `finetype-core`, lift to a CLI-internal free function, or
inline inside `cmd_validate_table` only?

**A:** Lift to a free function in `crates/finetype-cli/src/main.rs`,
parameterised on `try_wrap: bool`. Both `cmd_validate_table` (this
card) and `cmd_load` call it; when `cmd_load` is removed (Q4), the
helper survives in CLI scope. Don't promote to `finetype-core`: SQL
emission is currently CLI-local (`sql_ident`, `sql_quote`,
`format_column_name` all live in CLI), and the only consumer outside
CLI today is the DuckDB extension which emits its own SQL via the
loadable-extension API, not string templates.

### Q3: `--no-transform` escape hatch?

**Q:** Memo flags an edge case — user wants raw VARCHAR output. Add a
`--no-transform` flag, hide it, or refuse?

**A:** Refuse. SQL after the fact is the right escape hatch. With
`x-finetype-label` round-tripped into `expected_type` in the reject
sidecar, `SELECT CAST(col AS VARCHAR) FROM orders` is one line and
locally inspectable. The Meridian "spark joy for analysts" pillar
favours fewer flags over discoverable but rarely-used affordances. If
diagnostic users surface, document the SQL pattern in the README's
`validate` section instead of adding a flag.

### Q4: Migration path for `cmd_load`

**Q:** Hide `cmd_load` and warn? Remove outright in v0.6.19? Keep one
release of overlap?

**A:** Remove outright in v0.6.19. Matches the hard-removal posture
that PR #51 ratified (visibility-cleanup spec line 16: "Scripts
pinning old surfaces fail loudly rather than drift silently"). No
internal callers — `make ci` doesn't use it, no training script does,
no test does outside golden tests for `load` itself. Migration is a
one-liner: `finetype load file.csv -t orders` becomes `finetype
validate file.csv schema.json --db out.db --table orders` (with
schema.json from `finetype profile -f file.csv -o json-schema`).
~270 LOC deleted from `main.rs:3061-3328`.

### Q5: Schema lacks `x-finetype-label` on a column

**Q:** Today `SchemaExtensions::extract` returns `(None, None)` for
unlabelled columns and validation gracefully passes through. After
folding, what's the projection rule? Pass-through, drop, or refuse?

**A:** Bare quoted identifier — VARCHAR pass-through. Matches today's
`build_load_expr` VARCHAR branch (`main.rs:3344`) and the existing
graceful-degradation contract from MADR 0064 (spec ac-11 at
`2026-04-22-duckdb-extension-ergonomics/spec.yaml:259`). Existing
test `vrp_ac11_null_on_absence` continues to assert NULL
`expected_type`/`type_confidence` in the reject sidecar AND must now
also assert the user table contains the column as VARCHAR.

### Q6: MCP `validate` tool — gain materialise mode?

**Q:** Mirror the CLI's new typed-CTAS shape in MCP, or stay engine-only?

**A:** Stay engine-only for v0.6.19. The visibility-cleanup spec
already pinned MCP as out-of-scope for this rally (constraint line 19);
expanding now would re-open the carve-out. MCP today exposes the right
primitives for an agent driving the typed-output flow: `validate`
(engine), `ddl` (typed-output SQL), `profile` (now gaining
`-o json-schema`). Materialise + transforms in MCP rides the post-rally
"MCP surface audit" follow-up. `ValidateRequest` shape unchanged
(`crates/finetype-mcp/src/tools/validate.rs:9-22`).

---

## Summary

### Goal

Make `finetype validate --db --table` write a typed user table by
applying per-column transforms inside the same DuckDB session that
performs validation. Surface transform failures via a new
`'TRANSFORM_FAILED'` reject ontology entry. Remove `cmd_load`. Single
verb for the import flow: `profile → validate → done`.

### Constraints

1. **TRY-wrap transforms** — single-CTAS shape preserved, no second
   validation pass.
2. **New ontology values**: `error_type = 'TRANSFORM_FAILED'` and
   `constraint_failed = 'transform'`. MADR 0064's `finetype_reject_errors`
   schema extends; existing 13 columns and existing values unchanged.
3. **Projection builder lives in `finetype-cli/src/main.rs`** as a free
   function; no `finetype-core` dependency added.
4. **No `--no-transform` flag** — SQL CAST is the documented escape
   hatch.
5. **`cmd_load` removed outright in v0.6.19** — `Load` Commands variant,
   dispatch arm, function body all deleted (~270 LOC).
6. **Unlabelled columns pass through as VARCHAR** — graceful-degradation
   contract from MADR 0064 ac-11 preserved.
7. **MCP `validate` tool unchanged** — engine-only, no materialise mode.
8. **Existing 15 `vrp_*` tests stay green** — they all assert with
   `--db`/`--table` supplied (now writing typed columns instead of
   VARCHAR); per-test assertions for column types may need updating.
9. **Existing check-only mode (PR #51) preserved** — when `--db`/
   `--table` are absent, no transforms run, behaviour unchanged.
10. **Hard-removal posture** — `finetype load …` errors via clap
    unknown-subcommand handler; CHANGELOG carries the migration line.

### Success Criteria

- `finetype validate file.csv schema.json --db out.db --table orders`
  produces:
  - `orders` table in `out.db` with per-column transforms applied
    (typed columns, e.g. `DATE`, `DECIMAL(18,2)`, lowered emails)
  - `finetype_reject_errors` sidecar with reject rows for both
    schema rejections AND transform failures
- A row whose value matches `pattern` but fails `strptime` lands in
  `finetype_reject_errors` with `error_type = 'TRANSFORM_FAILED'` and
  `constraint_failed = 'transform'`. Excluded from user table.
- Schema columns lacking `x-finetype-label` produce VARCHAR columns in
  the user table; no error, no warning.
- `finetype load file.csv …` errors with clap's unknown-subcommand
  message.
- All 15 existing `vrp_*` tests pass after migration; `vrp_ac11_*`
  extended to assert VARCHAR pass-through column type.
- 2-4 new `vrp_*` tests covering: TRY-wrap transform success,
  TRY-wrap transform failure (`'TRANSFORM_FAILED'` reject row),
  NULL-in-NULL-out (no false transform-failure), end-to-end pipeline
  parity vs old `validate + load` chain.
- `cmd_load`'s ~270 LOC removed from `main.rs`; net diff negative.
- Exit codes preserved: 0 = no rejects (including transform failures),
  1 = rejects present, 2 = error.

### Decisions Surfaced

- **D1 — Transform-failure surfacing**: TRY-wrap + new error_type
  `'TRANSFORM_FAILED'` + new constraint_failed `'transform'`. Splits
  rather than overloads `'SEMANTIC_TYPE'`.
- **D2 — Projection-builder home**: lift to free function in CLI
  `main.rs`. No `finetype-core` API expansion.
- **D3 — `--no-transform` escape hatch**: refuse the flag; SQL CAST
  documented in README.
- **D4 — `cmd_load` migration**: remove outright in v0.6.19. Hard-
  removal posture consistent with PR #51.
- **D5 — Unlabelled columns**: VARCHAR pass-through via bare quoted
  identifier; preserves MADR 0064 ac-11.
- **D6 — MCP `validate` tool**: engine-only; materialise mode rides
  the post-rally MCP audit follow-up.

→ MADR write-up needed: extension to MADR 0064's reject ontology
(transform-failed value space). Likely as MADR 0071 (after card 0006's
0070), refining 0064. Or as a smaller addendum inline in the spec.
Decide during spec authoring.

### Implementation Notes

- Today's CTAS: `main.rs:3884` —
  `"CREATE TABLE {} AS SELECT * EXCLUDE(__row_idx) FROM {} {};"`
- New CTAS shape (sketch from memo §"Implementation diff"):
  ```rust
  let projection = build_transform_projection(&headers, &extensions, &taxonomy, /*try_wrap=*/true);
  format!(
      "CREATE TABLE {} AS SELECT {} FROM {} {};",
      user_table_ident,
      projection,
      sql_ident(&staging_ident),
      valid_filter,
  )
  ```
- Reject-detection pass: after CTAS, `SELECT __row_idx, col FROM
  __finetype_staging WHERE col IS NOT NULL AND TRY(transform) IS NULL`
  per transformed column to find transform-failed cells; INSERT into
  `finetype_reject_errors`. Or fold into a single CTE. Spec author
  picks the SQL shape.
- `finetype_reject_errors` columns unchanged (REJECT_SIDECAR_DDL at
  `main.rs:3713-3728`); only the value spaces of `error_type` and
  `constraint_failed` extend.
- `cmd_load` deletion: `Commands::Load` variant (~`main.rs:225-265`),
  dispatch arm (`main.rs:631`), `cmd_load` function
  (`main.rs:3061-3328`), `build_load_expr` (`main.rs:3336-3353` —
  consumed by the lifted helper).
- CSV reader's NULL-normalisation (`main.rs:3804-3812`) means staging
  NULL needs careful handling: `staging IS NOT NULL AND TRY(transform)
  IS NULL` is the failure predicate; `staging IS NULL AND TRY(transform)
  IS NULL` is legitimate.
- `valid_row_indices` set: rows that fail transform are removed from
  this set before INSERT into the user table. Two implementation
  options (memo's open question) — single-CTAS-with-WHERE-NOT-IN-failed
  set, or staging-temp-table-then-conditional-INSERT. Spec author picks.
- `constraint_value` for transform-failed rows: carry the transform
  expression (mirrors `pattern`-token semantics where `constraint_value`
  is the regex). `error_message` carries the failing input value.

### Open Questions

- **MADR target**: extend MADR 0064 inline, or write a fresh MADR
  refining it (suggesting 0071 after card 0006's 0070)? Spec author's
  call.
- **Reject SQL shape**: separate post-CTAS pass per transformed column,
  or single CTE detecting all transform failures at once? Trade-off is
  SQL complexity vs round-trips. Spec.
- **Existing `vrp_*` test migration**: which assertions tighten (column
  types now non-VARCHAR), which stay (reject ontology). Implementation
  detail.
- **`cmd_load` golden tests** (`crates/finetype-cli/tests/cli_golden.rs`
  — `golden_load_*`): replace with `golden_validate_typed_*` or delete
  if `validate_cli.rs` (15 tests at `crates/finetype-cli/tests/`)
  already covers the surface. Implementation.
