---
status: accepted
date-created: 2026-04-28
date-modified: 2026-04-28
---
# 0070. Schema verb folds entirely — type-mode → taxonomy, table-mode → profile

## Context and Problem Statement

`finetype schema` carried two unrelated jobs:

- **Type-mode** (`schema KEY`) — emit a JSON Schema document for a single
  type or glob from the bundled taxonomy. Plain dictionary lookup, no
  inference, no I/O beyond reading `labels/`.
- **Table-mode** (`schema FILE.csv`) — profile a CSV/Parquet file and emit
  a column-level JSON Schema. Full inference pipeline (model load,
  sibling-context attention, per-column classification) ending in a
  schema build.

The two jobs share nothing operationally. Decision 0031 (March 2026)
fused them into a single verb under the heuristic "if the argument
looks like a file path, route to table-mode; otherwise type-mode." The
heuristic worked but the fused verb obscured the divergent behaviour —
two completely different code paths under one help string, with flag
collisions (`--stats`, `--stdout`, `--enum-threshold` only meaningful
for table-mode) and a CLI surface entry that needed a paragraph of
clarification rather than a sentence.

The v0.6.19 CLI consolidation rally (memos
`2026-04-27-schema-profile-overlap`, `2026-04-27-mcp-surface-audit`,
PR #51 visibility-cleanup) shipped the framing that the public CLI
surface should mirror the MCP tool surface and that each verb should do
one thing. Card 0003 (`profile -o json-schema`, PR #53) gave table-mode
a natural home on the `profile` verb. Card 0006 — this decision —
finishes the fold by giving type-mode a natural home on `taxonomy`.

## Considered Options

1. **Rename only** — keep `schema` but split into `schema type` and
   `schema table` sub-subcommands.
2. **Soft deprecation** — emit a warning when `schema` is invoked,
   redirect to the new verbs internally, remove after one release.
3. **Hard fold** — delete `Commands::Schema` entirely in v0.6.19, route
   type-mode through `taxonomy KEY -o json-schema` (card 0006), route
   table-mode through `profile -f FILE -o json-schema` (card 0003,
   PR #53). No dispatch shim, no warning-only branch.

## Decision Outcome

Chosen option: **"Hard fold"**, because:

- v0.6.19 is a CLI surface consolidation release — the visibility-cleanup
  spec (PR #51) already established the posture of removing dead
  affordances rather than wrapping them in deprecation cycles.
- The companion table-mode migration in card 0003 (PR #53) already
  shipped without a deprecation shim. A soft deprecation on type-mode
  would have asymmetric semantics across the two halves of the same
  fold.
- Type-mode export is genuinely a taxonomy operation: `taxonomy` already
  filters definitions by domain/category/priority and emits per-type
  views in plain/json/csv. Adding a `json-schema` output format is the
  natural extension. Card 0006 also gives `taxonomy` a positional KEY
  argument (with the same exact-match-or-glob predicate previously
  bound to `schema`), so the migration is `schema KEY` →
  `taxonomy KEY -o json-schema` — one-for-one.
- The MCP `schema` tool's type-key branch is **retained for v0.6.19**
  per the visibility-cleanup carve-out (memo `2026-04-27-mcp-surface-audit`
  line 116). MCP is not part of this fold; the audit ships in v0.6.20.
  The temporary asymmetry is marked in source via comments in
  `crates/finetype-mcp/src/lib.rs` and `crates/finetype-mcp/src/tools/schema.rs`.

The deletion happens in one PR alongside README, CLAUDE.md, and
`.claude/skills/*` migrations and a CHANGELOG entry. MADR 0031 is
flipped to `superseded by 0070`. The shared helper module at
`crates/finetype-mcp/src/json_schema.rs` (established by card 0003)
gains an `emit_type_schema(label, def)` function so both surfaces (CLI
type-mode + MCP `schema` type-key branch) route through the same
emitter.

### Consequences

- **Good** — public CLI surface drops from 7 to 6 commands (`infer`,
  `profile`, `validate`, `load`, `mcp`, `taxonomy`). Each verb does one
  thing. The `-f, --file` flag collision under `Commands::Schema`
  (taxonomy directory vs CSV path) goes away with the verb.
- **Good** — type-mode export gains the verbosity-contract symmetry from
  PR #51. The pre-existing `cmd_schema` only emitted `x-finetype-pii`;
  the new `taxonomy KEY -o json-schema` emits BOTH `x-finetype-label`
  and `x-finetype-pii`, matching `emit_table_schema`'s contract. This
  IS a behaviour change in the JSON Schema export contract — recorded
  explicitly under the v0.6.19 CHANGELOG "Changed" sub-bullet so
  downstream consumers see it.
- **Good** — both emitters share one module (`finetype-mcp/src/json_schema.rs`).
  No `finetype-core` API expansion in v0.6.19 (constraint inherited
  from card 0003); future archaeologists looking for the type-mode
  emitter find it next to the table-mode emitter, with both governed
  by the same verbosity-contract module docs.
- **Good** — the always-array output shape (even for single-match
  positional KEYs) matches `taxonomy`'s other output formats and means
  downstream tooling can `serde_json::from_str::<Vec<Value>>` without
  branching on cardinality.
- **Bad** — users with shell aliases or scripts using `finetype schema KEY`
  break at upgrade. Mitigations: clap surfaces the standard
  "unrecognized subcommand" error (exit 2), the CHANGELOG entry carries
  the verbatim migration map, and the fold is documented in the
  v0.6.19 release notes. v0.6.19 is a minor-version surface
  consolidation, not a 1.0 stability release; same posture as PR #51's
  hard removal of `--model` and `--sharp-only`.
- **Bad** — temporary CLI/MCP asymmetry until v0.6.20 (MCP `schema`
  tool's type-key branch survives). Source comments mark the asymmetry
  so it stays visible until the audit lands.

### References

- Card 0006 (`orbit/cards/0006-command-line-interface.yaml`) — feature
  card for the CLI surface consolidation.
- Spec (`orbit/specs/2026-04-28-schema-verb-fold/spec.yaml`) — 11 ACs,
  v1.1 after cycle-2 review-spec APPROVE.
- Memo `2026-04-27-schema-profile-overlap.md` — original case for
  folding both halves of `schema` onto their natural homes.
- Memo `2026-04-27-mcp-surface-audit.md` — MCP carve-out and v0.6.20
  follow-up framing.
- PR #51 (CLI visibility cleanup, v0.6.19 — established the verbosity
  contract for table-mode).
- PR #53 (card 0003, `profile -o json-schema` — table-mode migration,
  established the shared helper module).
- MADR 0031 — superseded by this decision; the fused-verb framing was
  the right call in March 2026, but the surface consolidation rally
  surfaced its costs.
