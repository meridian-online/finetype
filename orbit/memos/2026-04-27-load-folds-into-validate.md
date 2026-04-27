# Memo: `load`/`cast` folds into `validate` — supersedes the prior load-rethink memo

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — supersedes `2026-04-27-load-rethink-as-transform.md`
**Tags:** cli, load, validate, transform, pipeline

## What changed in my thinking

The prior memo (`load-rethink-as-transform`) proposed pivoting `load`
to a new `cast` verb that operates on existing DuckDB tables. Hugh's
counter: **just fold it into `validate`.** That's stronger. Writing
this memo to explain why.

## Today's `validate` is one taxonomy lookup away from doing transforms

Look at the SQL `cmd_validate_table` already emits (`main.rs:3958`):

```sql
CREATE TABLE orders AS
SELECT * EXCLUDE(__row_idx) FROM __finetype_staging_<uuid>
WHERE __row_idx IN (valid_indices);
```

Everything in the user table is VARCHAR — that's the whole staging
shape (`read_csv(..., all_varchar=true)`, `main.rs:3931`). Valid rows
get copied through as text. To use the data, the user must then run a
separate `load`/`cast` step that re-reads the schema, looks up the
transform per column, and emits a *second* CTAS.

The schema already carries `x-finetype-label` per column. The
transform for that label is in the taxonomy. So the validate code path
is **already holding everything it needs to apply the transforms** —
it just doesn't.

Replace the projection with per-column transforms keyed by label:

```sql
CREATE TABLE orders AS
SELECT
    "order_id",                                                   -- no transform → VARCHAR passthrough
    LOWER(CAST("customer_email" AS VARCHAR)) AS "customer_email",
    strptime("order_date", '%Y-%m-%d')::DATE AS "order_date",
    CAST(REGEXP_REPLACE(... "total_price" ...) AS DECIMAL(18,2)) AS "total_price",
    ...
FROM __finetype_staging_<uuid>
WHERE __row_idx IN (valid_indices);
```

Same SQL shape `cmd_load` produces today. Same SQL session validate
already shells out to. Zero new infrastructure.

## The pipeline becomes two verbs

Before this memo:

```
profile  → discover types + emit JSON Schema
validate → enforce schema, partition valid/reject (rows stay VARCHAR)
cast     → apply transforms, write typed table
```

After:

```
profile  → discover types + emit JSON Schema
validate → enforce schema, partition valid/reject, apply transforms (typed table)
```

Two operational verbs. `schema` (type-mode only) stays alongside as
the taxonomy export. **That's the whole user-facing surface for the
import / clean-up flow.**

This matches the Meridian pillar — write programs that do one thing
and do it well — better than the three-verb chain. "Validate" in this
shape means "make this data fit for use." The reject sidecar is the
validation report; the typed user table is the prepared data. Both
fall out of the same SQL session.

## Why folding is right (and not just convenient)

```
| Concern                            | Three-verb shape                             | Folded shape                                  |
|------------------------------------|----------------------------------------------|-----------------------------------------------|
| User intent ("clean my data")      | three commands, two intermediate artefacts   | one command, one .db                          |
| Schema is the contract             | each verb re-reads schema                    | schema read once                              |
| Transform contract per type        | applied in cast, ignored in validate         | applied where the data is being written       |
| Reject reasons (data wrong vs fit) | split across two outputs                     | unified in `finetype_reject_errors` (extend)  |
| Performance                        | two duckdb sessions, two CTAS                | one session, one CTAS                         |
| What to do with valid VARCHAR rows | open question — analyst writes a third step | answered — they're already typed             |
```

Today's middle artefact (validate's VARCHAR user table) is a *waste
shape*. No one wants VARCHAR-only valid rows; they want valid + typed.
We were forcing the user to do a second step to reach the obvious
outcome.

## What about transform failures?

This is the one new wrinkle. A row passing `pattern: ^\d{4}-\d{2}-\d{2}$`
should always parse as a date — but `strptime("2024-02-30", '%Y-%m-%d')`
will fail (no Feb 30). Today's `cmd_load` ignores this; the SQL crashes
or silently NULLs depending on DuckDB's behaviour.

In folded `validate`, a transform failure on a row that passed
validation is a real signal — one of:

- **Bug in the type's validation block** — the validator accepted a
  value the transform can't handle. Tighten the regex.
- **Bug in the transform** — pattern is right, transform is wrong.
- **Edge case in DuckDB SQL** — rare but possible.

All three deserve to surface, not silently drop. Two implementation
options:

- **Use `TRY_CAST` / `TRY(...)` style wrappers** in the transform
  projection. Failures emit NULL; a follow-up `WHERE col IS NULL` on
  rows where staging was non-null detects them. New constraint token
  in `finetype_reject_errors`: `constraint_failed = 'transform'`.
- **Run validation a second time post-transform.** Slow; not necessary.

Recommendation: TRY-wrap the transform in the projection; emit a
`TRANSFORM_FAILED` reject record per failed cell. Mirrors the existing
`SEMANTIC_TYPE` `error_type` (`main.rs:3975`) — same shape, different
token. The constraint surface stays consistent: the analyst sees
"this row passed pattern but the transform produced NULL," and that's
exactly the diagnostic they need.

## Edge cases

**1. User wants raw VARCHAR output.** Rare, but: e.g., feeding a
downstream tool that expects strings. Two answers:

- Add a `--no-transform` flag. Falls back to today's pass-through
  projection.
- Don't add the flag. The user can always `SELECT CAST(col AS VARCHAR)
  FROM orders` after. SQL is already there for them.

Recommend: don't add the flag. SQL is the answer when SQL is the
context.

**2. Type was inferred but no transform exists.** A few taxonomy
entries don't ship a `transform` field (passthrough types). Today
`cmd_load` handles this (`-- {type}` comment, no projection
modification). Folded validate does the same.

**3. Schema lacks `x-finetype-label`.** The `SchemaExtensions::extract`
path (`main.rs:3700`) already gracefully handles this (NULL
`expected_type`). For folded validate, columns without an
`x-finetype-label` would simply pass through VARCHAR. Same graceful
degradation.

**4. The user table is the typed one — what was the prior name for
"valid VARCHAR"?** It's gone. There's no intermediate. The reject
sidecar still references row indices in the staging (or in the input
file via `line`), not the user table — so reject auditing is
unaffected.

## What about the read-only check mode?

The `validate-required-flags` memo proposed making `--db`/`--table`
optional. That stays orthogonal:

```bash
# Read-only check (no .db, no transforms applied)
finetype validate raw.csv raw.schema.json

# Full pipeline (validate + transform + write)
finetype validate raw.csv raw.schema.json --db data.db --table orders
```

When no .db, nothing's written, so no transforms run — just the
pass/fail engine. When .db is supplied, the user gets validated +
typed in one shot.

## What about "I want types but I trust my data"?

Today some users would run `finetype load file.csv` straight to types
without validating first — fast path, no schema needed. After folding,
that path doesn't exist as a separate verb.

Two reframings:

- **Profile + validate is the answer.** `finetype profile -f file.csv
  -o json-schema --stdout > s.json && finetype validate file.csv s.json
  --db data.db --table orders` — three commands, but the schema is
  durable, the validation is a free extra signal, and the typed table
  is the same outcome.
- **Add an `--infer-schema` flag to validate.** When passed, validate
  runs profile internally to derive the schema before validating.
  Single command, same outcome. Heavier internal coupling but
  user-facing simplicity.

Recommendation: ship the chain first. If users complain about the
two-step flow for the no-schema case, add `--infer-schema` later. The
chain is already short (two commands) and produces a durable schema
the user usually wants anyway.

## Naming

"Validate" still fits — it's the verb for "make this data fit for
use." The implementation grew to include transforms; the user-facing
intent is unchanged: I have data and a schema, give me the clean
typed output.

If naming matters, three honest alternatives:

- `validate` — keep it. Broadens the implied scope slightly, but the
  command's *purpose* (data + schema → clean output) is unchanged.
- `prepare` — describes the outcome better. Loses recognition.
- `import` — emphasises the loading aspect. Loses the
  validation-first framing.

Pick `validate`. Existing scripts and docs continue to work. The
behaviour is a strict superset of today's.

## Implementation diff (sketch)

In `cmd_validate_table` (`main.rs:3820+`), the current SELECT projection
at line 3952/3959:

```rust
"INSERT INTO {} SELECT * EXCLUDE(__row_idx) FROM {} {};"
"CREATE TABLE {} AS SELECT * EXCLUDE(__row_idx) FROM {} {};"
```

Becomes:

```rust
let projection = build_transform_projection(&headers, &extensions, &taxonomy);
// "col1, LOWER(CAST(col2 AS VARCHAR)) AS col2, strptime(col3, ...)::DATE AS col3, ..."

format!(
    "CREATE TABLE {} AS SELECT {} FROM {} {};",
    user_table_ident,
    projection,
    sql_ident(&staging_ident),
    valid_filter,
)
```

`build_transform_projection` already exists in spirit in `cmd_load`
(around `main.rs:5034+`). Lift it to a shared helper, parameterise
on whether to TRY-wrap (for validate's reject-collecting path), call
from both sites until `cmd_load` is removed.

## Roadmap shift

Updating the dependency ordering from the prior memo:

1. `profile -o json-schema` lands (folds schema into profile).
2. `validate` gains read-only mode AND fold transforms into the
   write path.
3. `cmd_load` deprecated → removed (no replacement; `validate`
   covers it).
4. Hide-it group ships in the same release.

The user-facing CLI after v0.7.0:

```
infer | profile | validate | schema | mcp
                                       (+ taxonomy for browsing)
```

**Five verbs.** Down from eleven today. Each does one thing.

## Supersedes

This memo supersedes `2026-04-27-load-rethink-as-transform.md`. Mark
that one as `superseded by 2026-04-27-load-folds-into-validate` if we
keep both, or delete it if we want the trail clean. Recommend keeping
both — the cast variant is a real alternative and the comparison is
useful when the spec is written.

## Not action yet

Observation memo. This is the highest-leverage of today's ten memos —
it collapses two verbs into one and removes a wasted intermediate
artefact. Wants a discovery / spec when ready.
