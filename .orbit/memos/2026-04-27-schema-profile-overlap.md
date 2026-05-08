# Memo: `schema` and `profile` overlap — fold table-mode schema into profile

**Date:** 2026-04-27 (revised same day — see "Both modes fold" below)
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a direction
**Tags:** cli, schema, profile, taxonomy, ux

> **Revision (same day):** the original memo proposed folding only
> table-mode `schema` into `profile`, leaving type-mode `schema KEY`
> as a standalone verb. Hugh's stronger counter — type-mode is a
> taxonomy export, so it belongs under `taxonomy`. Both modes fold;
> the `schema` verb disappears entirely. See "Both modes fold" below
> for the revised shape. The original analysis is retained for the
> alternative-comparison trail.

## What both commands actually do

Run the same inference engine on a CSV; emit per-column type
predictions in different shapes.

**`profile -o json`** — per-column inference output:

```json
{
  "broad_type": "VARCHAR",
  "column": "customer_email",
  "confidence": 1.0,
  "transform": "LOWER(CAST({col} AS VARCHAR))",
  "type": "identity.person.email"
}
```

**`schema <file.csv>`** — same predictions, dressed as JSON Schema:

```json
"customer_email": {
  "maxLength": 254,
  "pattern": "^[a-zA-Z0-9.!#$...]$",
  "type": "string",
  "x-finetype-label": "identity.person.email",
  "x-finetype-confidence": 1.0,
  "x-finetype-transform": "LOWER(CAST({col} AS VARCHAR))",
  ...
}
```

Both call into `finetype-core::table_validator` / multi-branch via the
same code paths. The pipeline is identical; only the projection differs:

```
| Command       | Inference engine | Per-column shape           | JSON Schema validators? |
|---------------|------------------|----------------------------|-------------------------|
| profile -o json | multi-branch    | flat metadata              | no                      |
| schema <file>   | multi-branch    | JSON Schema property       | yes (pattern, length…)  |
```

This is one feature — column-level type inference — wearing two CLI
hats.

## The proposal

Make JSON Schema an output format of `profile`:

```bash
finetype profile -f file.csv -o json-schema    # writes file.schema.json
finetype profile -f file.csv -o json           # current behaviour
finetype profile -f file.csv -o markdown       # current behaviour
```

Retire **table-mode** `schema`. Type-mode `schema` (`finetype schema
identity.person.email`) stays — it's a different operation.

## Why type-mode and table-mode are genuinely different

```
| Mode        | Input                  | Operation                                    | Inference? |
|-------------|------------------------|----------------------------------------------|------------|
| Type-mode   | type key (e.g. email)  | export taxonomy → JSON Schema                | no         |
| Table-mode  | CSV file               | infer types → JSON Schema with predictions   | yes        |
```

Type-mode is a **taxonomy export** — deterministic from `labels/*.yaml`.
It has no relationship to inference. Table-mode is **inference output**
in JSON Schema clothes. Conflating them under one verb (`schema`) is
the same mistake the `--file` flag-collision memo flagged: same surface,
two different intents.

## What changes

**Add to `profile`:**

- New `OutputFormat::JsonSchema` variant.
- When chosen, write `<input>.schema.json` (current sidecar behaviour)
  with a `--stdout` toggle for piping.
- Carry over the `--stats` flag (observed-data constraints) and
  `--enum-threshold` from the current `schema` command.

**Retire on `schema`:**

- Remove the table-mode branch (the path-sniffer at `main.rs:642-667`
  goes away).
- The `<TYPE_KEY>` argument becomes type-key only — no more positional
  overloading.
- Side benefit: this cleanly resolves the
  `2026-04-27-schema-cli-flag-collision.md` memo. With the positional
  no longer overloaded, the `-f, --file` taxonomy-directory flag stops
  fighting users for a slot.

**Rename `schema` → ?**

Once table-mode is gone, "schema" is misleading — the command exports
a single type's JSON Schema from the taxonomy. Better names:

- `taxonomy export <KEY>` — fits with existing `taxonomy` subcommand
  shape.
- `schema export <KEY>` — keeps "schema" but disambiguates.
- Leave as `schema <KEY>` — least churn; clearest if we add a
  one-liner clarifying "type-level only."

Recommendation: leave the verb, fix the doc. Renaming a public command
is a bigger break than the substantive change (folding table-mode into
profile).

## What this does to the MCP server

`crates/finetype-mcp/src/tools/schema.rs` exposes both modes through
one MCP tool. The same logic applies: split the tool into:

- `taxonomy_schema` (or just keep current `schema` for type-mode)
- `profile` already exists for table-mode and can grow a
  `format: "json-schema"` parameter.

The MCP tool list shrinks net by zero — same capabilities, cleaner verbs.

## Migration

- Add `--output json-schema` to profile.
- Deprecate (warn but allow) `schema <file.csv>` for one release.
- Remove in v0.7.x.
- CHANGELOG: "Folded table-mode `schema` into `profile -o json-schema`.
  Type-mode `schema <KEY>` unchanged."

## Stacks with prior memos

This is the sixth CLI ergonomics observation from today. Specifically:

- Resolves the **flag-collision** memo as a side-effect (positional
  is no longer overloaded).
- Composes naturally with the **schema-export-verbosity** memo
  (whatever fields we keep on table-mode JSON Schema, they live under
  `profile -o json-schema`).

## Both modes fold (revised)

The original memo above kept type-mode `schema KEY` as a standalone
verb. That assumption doesn't hold up under scrutiny — type-mode is
"give me JSON Schema for these taxonomy types," which is the same
operation `taxonomy` already performs for plain/json output. JSON
Schema is just one more output format on a verb that already does
type inspection.

So both modes fold to their natural sibling:

```
| Mode        | Today                              | After fold                                |
|-------------|------------------------------------|-------------------------------------------|
| Type-mode   | finetype schema KEY                | finetype taxonomy KEY -o json-schema      |
| Table-mode  | finetype schema -f file.csv        | finetype profile -f file.csv -o json-schema |
```

**The `schema` verb disappears entirely.** Public CLI surface tightens
from five verbs to four:

```
| Before fold (5 verbs + taxonomy) | After fold (4 verbs + taxonomy)  |
|----------------------------------|----------------------------------|
| infer                            | infer                            |
| profile                          | profile (gains -o json-schema)   |
| validate                         | validate                         |
| schema                           | (folded away)                    |
| mcp                              | mcp                              |
| + taxonomy (browsing)            | + taxonomy (browsing + -o json-schema) |
```

This is the same pattern applied symmetrically: where a verb's "what"
is already covered by another verb and only the output shape differs,
the output shape is a flag on the existing verb, not a separate verb.

### Why type-mode also belongs as an output format, not a verb

`finetype taxonomy 'datetime.date.*'` already returns matching types
in plain or json output. Adding `-o json-schema` is mechanical —
emit the same set of types, dressed in JSON Schema clothing. The
emitter is shared with `profile -o json-schema` (and with table-mode
schema today); pulling it under taxonomy keeps the canonical types-
to-schema path in one place.

### What gets dropped from this memo's earlier "rename schema → ?" question

The earlier section's three rename options (`taxonomy export <KEY>`,
`schema export <KEY>`, leave-as-is) all become moot — the verb is
gone. Type-mode users migrate from `finetype schema KEY` to
`finetype taxonomy KEY -o json-schema`; table-mode users migrate to
`finetype profile -f file.csv -o json-schema`.

### Migration (revised)

- Add `-o json-schema` to **both** taxonomy and profile.
- Deprecate `finetype schema` (both modes) for one release; emit a
  warning pointing to the replacement verb.
- Remove `schema` verb entirely in the following release.
- CHANGELOG: "Folded `finetype schema` — type-mode → `finetype
  taxonomy KEY -o json-schema`, table-mode → `finetype profile -f
  file.csv -o json-schema`. The `schema` verb is removed."

### Composition with the schema-cli-flag-collision memo

The `--file`/`--taxonomy` flag-collision memo
(`2026-04-27-schema-cli-flag-collision.md`) proposed renaming
schema's `--file` flag to `--taxonomy` for consistency. With the
schema verb dying entirely, that rename is moot — the flag goes
away with the verb. That memo is therefore superseded by this one.

### Composition with schema-export-verbosity

The verbosity reduction (drop derivable `x-finetype-*` fields, keep
label + pii) lives in the JSON Schema *emitter*, which is shared
across all callers (CLI schema today; CLI taxonomy and profile after
fold; MCP `schema` tool today; MCP taxonomy + profile tools after
fold). The verbosity reduction can land independently of the verb
fold — they're orthogonal changes.

## Not action yet

Observation memo. Together with `validate-required-flags` and
`load-folds-into-validate`, this is the architectural shape change
of the CLI polish:

```
infer    → classify values
profile  → discover types (multiple output shapes including JSON Schema)
validate → enforce types + transform (read-only summary OR materialise to .db)
taxonomy → browse + export (multiple output shapes including JSON Schema)
mcp      → server
```

Four public verbs (plus taxonomy for browsing). Each does one thing.
