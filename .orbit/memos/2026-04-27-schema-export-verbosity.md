# Memo: `finetype schema` export verbosity

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a direction
**Tags:** cli, schema, export, dx

## What's emitted today

A table-level schema column (e.g., `credit_card_last4` from
`ecommerce_orders.csv`) currently looks like this:

```json
"credit_card_last4": {
  "maxLength": 10,
  "minLength": 3,
  "type": "string",
  "x-finetype-broad-type": "VARCHAR",
  "x-finetype-confidence": 0.9800000190734863,
  "x-finetype-domain": "geography",
  "x-finetype-label": "geography.address.postal_code",
  "x-finetype-pii": false,
  "x-finetype-transform": "CAST({col} AS VARCHAR)"
}
```

Six `x-finetype-*` fields per column, plus the standard JSON Schema
`type` / `minLength` / `maxLength` / `pattern` / `enum` validators
inherited from the taxonomy definition.

(Type-level schema — `finetype schema identity.person.email` — is a
slightly different beast: it adds `x-finetype-key`,
`x-finetype-format-string`, and `x-finetype-transform-ext`. Treated
separately at the bottom.)

## Hugh's proposal

Drop everything except `x-finetype-label` and `x-finetype-pii`.

## Where each field is emitted and who consumes it

```
| Field                      | Emitted at        | Consumed by                                                                                          | Derivable?                  |
|----------------------------|-------------------|------------------------------------------------------------------------------------------------------|-----------------------------|
| x-finetype-label           | main.rs:2987,3008 | validate path (main.rs:3700) → reject sidecar `expected_type`                                        | no — primary identity        |
| x-finetype-pii             | main.rs:3004,3015 | none in code                                                                                          | yes (taxonomy lookup)       |
| x-finetype-confidence      | main.rs:2991,3012 | validate path (main.rs:3705) → reject sidecar `type_confidence` (nullable, graceful-degrades)        | no — run-time signal         |
| x-finetype-domain          | main.rs:2989,3010 | none                                                                                                  | yes — `label.split('.')[0]` |
| x-finetype-broad-type      | main.rs:2996      | none in column path (used only in MCP type-mode markdown summary, schema.rs:152)                     | yes (taxonomy lookup)       |
| x-finetype-transform       | main.rs:2999      | none                                                                                                  | yes (taxonomy lookup)       |
| x-finetype-format-string   | main.rs:3002      | none                                                                                                  | yes (taxonomy lookup)       |
```

Inventory by sweep over `crates/`. The MCP server
(`crates/finetype-mcp/src/tools/schema.rs`) duplicates the same emission
logic and would need the same trim.

## Why the proposal is right (mostly)

**Drop the derivables.** Domain, broad-type, transform, format-string
are all functions of the label — anyone with the taxonomy can recover
them. Embedding them in every column wastes bytes and creates two
sources of truth (the schema and the taxonomy) that can drift if the
schema is preserved across a taxonomy revision. Keep the label, derive
the rest on demand.

**Keep `x-finetype-pii`.** This *is* derivable from the label, but it's
a security/compliance signal. A consumer scanning a schema for "is this
column sensitive?" should not need to resolve the taxonomy to find out.
Embedding it in the schema makes the schema file self-sufficient as a
data-classification artefact. Strong instinct — keep it.

**Keep `x-finetype-label`.** Primary identity. Not negotiable.

## The one wrinkle: `x-finetype-confidence`

This is the only field in the current set that **isn't** derivable from
the label. It's a run-time inference signal captured at schema-authoring
time. The validate path reads it (`main.rs:3705`) and surfaces it as
`type_confidence` in the reject sidecar — useful when a downstream
analyst is triaging rejects ("was the type prediction confident? then
the schema is probably right and the data is wrong; was it shaky? maybe
the schema is the problem").

**The schema is meant to be a contract; confidence is run-time
provenance.** Mixing the two violates separation of concerns — once a
schema is committed, the original confidence is no longer meaningful
(the schema either holds or it doesn't). The reject sidecar's
`type_confidence` column is already declared nullable and the comment
at `main.rs:3688-3689` says "missing entries are allowed — the
corresponding reject columns render as NULL (graceful degradation)."
So dropping it doesn't break validate; it just means `type_confidence`
will be NULL for new schemas going forward.

**Two options if we want to preserve provenance:**

- **Drop entirely (Hugh's proposal).** `type_confidence` becomes always-NULL
  for new schemas; old schemas still populate it during their lifetime.
- **Move to a top-level `x-finetype-meta` block** (one per schema, not per
  column) capturing `{generated_at, model, version}` for the whole table —
  separates contract from provenance without burying it per-column.

Default to drop entirely. If we miss it, add the meta block in a
follow-up.

## Final shape

```json
"credit_card_last4": {
  "maxLength": 10,
  "minLength": 3,
  "type": "string",
  "x-finetype-label": "geography.address.postal_code",
  "x-finetype-pii": false
}
```

Five lines instead of nine. Roughly half the bytes. Everything still
recoverable with the label + taxonomy. Validate still works (confidence
gap-fills to NULL).

(Aside: the example also surfaces a misclassification — `credit_card_last4`
predicted as `postal_code` with `pii: false`. That's a separate problem,
not a schema-format problem.)

## Type-level schema — separate question

`finetype schema identity.person.email` (type-mode) emits a fuller
record because it's the canonical type definition, not the result of
column-level inference. Audience and intent differ:

- **Table-mode schema:** inference output, contract for one CSV. Should
  be lean — that's this memo.
- **Type-mode schema:** taxonomy export, used by MCP `schema` tool's
  markdown summary (consumes `x-finetype-broad-type`) and by anything
  that wants a portable type definition. Trimming this is a separate
  call — leave it alone for now.

## Migration

- **Backward compat:** old schemas keep working — validate's
  `SchemaExtensions::extract` reads any subset of the fields and
  gap-fills missing ones with NULL.
- **Tests to update:** `crates/finetype-cli/tests/cli_golden.rs` lines
  625, 630, 650 assert on `x-finetype-broad-type` and `x-finetype-pii`
  in **type-mode** (`finetype schema email`) — those still pass because
  type-mode is unchanged.
  `crates/finetype-cli/tests/validate_cli.rs` lines 85-86 use
  `x-finetype-label` and `x-finetype-confidence` in fixtures — confidence
  fixture stays valid (still tolerated), but a new test should cover the
  "confidence absent" path explicitly.
- **MCP server:** `crates/finetype-mcp/src/tools/schema.rs` table-mode
  branch (lines 226-280) needs the same trim. One PR, both crates.
- **CHANGELOG:** mark this as a breaking change to schema output (v0.7.0
  material — or another patch given the early-release framing we used
  for v0.6.18).

## Not action yet

Observation memo. Promote to a card when we decide it's worth shipping
— probably bundled with the type-mode question and the MCP trim.
