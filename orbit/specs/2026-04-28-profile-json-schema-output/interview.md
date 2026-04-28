# Design: `profile -o json-schema`

**Date:** 2026-04-28
**Interviewer:** Nightingale (lead, with author Hugh)
**Card:** `orbit/cards/0003-tabular-data-profiling.yaml`
**Rally:** `orbit/specs/2026-04-28-v0619-cli-consolidation-rally/`
**Sibling cards:** 0005 (validate absorbs load), 0006 (schema verb fold)
**Target release:** v0.6.19

---

## Context

**Card:** *Tabular data profiling* — 5 scenarios, goal: "CSV and Parquet
profiling with 6 output formats (plain, json, csv, markdown, arrow,
json-schema), null detection, enum threshold — and json-schema output
as the canonical bridge into validate."

**Prior specs:** `2026-04-27-cli-pipeline-reshape/` (planned, this card)
and historical profiling work back to `2026-03-12-validate-command/`.

**Gap:** Today `profile` emits 5 formats. The card adds `json-schema`
as the 6th — fully replacing the table-mode behaviour of `finetype
schema <file.csv>`. The verb fold itself lives in card 0006; this
card delivers the destination.

**Source memos:**
- `orbit/memos/2026-04-27-schema-profile-overlap.md` (revised "Both
  modes fold" section)
- `orbit/memos/2026-04-27-schema-export-verbosity.md` (already shipped
  in PR #51; trimmed schema export to label + pii)

---

## Q&A

### Q1: Output destination — sidecar by default, or stdout?

**Q:** Today `schema <file.csv>` writes a sidecar `<input>.schema.json`
by default and emits to stdout under `--stdout`. Today `profile -o
json` writes to stdout. The fold has to pick one. Which?

**A:** Stdout-by-default, no `--stdout` flag. Every `-o X` shape goes
to stdout uniformly; `profile -f file -o json-schema > file.schema.json`
is the universal pattern. Migration from `schema -f file --stdout >
file.schema.json` is a one-character delete; migration from `schema -f
file` (implicit sidecar) gains a `> file.schema.json`. The asymmetry of
having one output format silently write to disk while the rest go to
stdout is a worse mental model than the redirect.

### Q2: Sidecar filename convention

**Q:** Moot under Q1 (stdout means user picks the redirect target). For
the record: if a sidecar were produced, the name?

**A:** Moot. User-controlled via shell redirect.

### Q3: `--stats` carry-over

**Q:** Today `schema <file.csv> --stats` adds observed-data constraints
(min/maxLength, minimum/maximum, enum, x-finetype-null-rate, x-finetype-
cardinality). Carry it forward, drop it, or generalise across all
profile output formats?

**A:** Carry it forward, gated to `-o json-schema`. Add `stats: bool` on
`Profile`'s clap variant; when set with `-o json-schema`, the emitter
attaches the same observed-data constraints today's `cmd_schema_table`
produces. With other output formats, clap conflict (consistent with the
`--db`/`--table` pattern shipped in PR #51). Behaviour-preserving
migration; no capability lost.

### Q4: `--enum-threshold` interaction

**Q:** Profile and schema both have `--enum-threshold` (default 50)
today, controlling different things. Reuse one flag, or keep them
separate?

**A:** Single `--enum-threshold` flag on `profile`. Controls categorical
detection in plain/json/markdown/csv rendering AND the `enum` keyword
in JSON Schema output (when `--stats` is on). One value, one mental
model. The implementation is no-op — `cmd_profile` already has
`enum_threshold: usize` in scope.

### Q5: `finetype schema` behaviour after fold

**Q:** Should `schema` hard-error in v0.6.19, hard-error with a tailored
hint, or soft-deprecate with a warning?

**A:** Defer to card 0006 — that card owns the deletion mechanics. From
this card's vantage, the answer is "the verb is gone in v0.6.19, period."
Card 0006 picks the exact mechanism (clap unknown-subcommand error +
CHANGELOG entry; no dispatch shim).

### Q6: MCP `profile` tool — gain `format` parameter?

**Q:** MCP `profile` tool today emits a fixed JSON shape. Mirror the CLI
fold (gain `format: "json-schema"`), or stay CLI-only and rely on MCP
`schema` table-branch?

**A:** Mirror in the same PR. Add `format` enum parameter (default
`"json"`); when `"json-schema"`, project results through the same shared
JSON Schema emitter. The MCP `schema` tool's table-branch hard-errors
in the same PR pointing at `profile`. (The MCP `schema` tool's type-key
branch is card 0006's concern.) Lockstep with CLI is the load-bearing
principle; the visibility-cleanup MCP carve-out specifically named the
fold as the moment MCP rejoins.

### Q7: `OutputFormat` enum variant naming

**Q:** Adding the new variant: `JsonSchema`, `Schema`, or split per-verb?

**A:** `JsonSchema` (kebab `json-schema` on CLI). Explicit, no overload,
mirrors the spec it implements. Sibling card 0006 uses the same variant
on its taxonomy parse. clap's `value_enum` derive handles the kebab-case.

### Q8: Test surface migration

**Q:** Today `cli_golden.rs` has type-mode `golden_schema_email` and
`golden_schema_iso_date` (line 142, 638, 740) but no table-mode schema
golden tests. What goes in the test surface for the new code path?

**A:** Add 2-3 new tests under `INFER REGRESSION GUARDS`-style section
or fresh "PROFILE JSON SCHEMA OUTPUT" section:
- `golden_profile_json_schema_<fixture>`: round-trip — `profile -o
  json-schema` output validates against itself via `validate <csv>
  <schema.json>` (when card 0005 lands; if it doesn't, just assert
  output shape).
- `golden_profile_json_schema_stats_<fixture>`: `--stats` produces
  `minLength`, `maxLength`, `minimum`, `maximum`, `enum` keywords as
  appropriate.
- `golden_profile_json_schema_enum_threshold_<fixture>`: `--enum-
  threshold` controls when `enum` keyword is emitted.

Existing fixture CSVs (`ecommerce_orders`, `people_directory`,
`titanic`) cover the input shapes. Existing `golden_schema_*` tests
(type-mode) are card 0006's concern — they migrate to
`golden_taxonomy_json_schema_*`.

---

## Summary

### Goal

Add `OutputFormat::JsonSchema` to `finetype profile` — emitting
table-level JSON Schema (with `x-finetype-label` + `x-finetype-pii`
extensions, plus optional `--stats` observed-data constraints) to
stdout. Mirror in MCP `profile` tool. Replaces the table-mode of
`finetype schema <file.csv>` byte-for-content (file shape unchanged
beyond verbosity reduction shipped in PR #51).

### Constraints

1. **Stdout-by-default for `-o json-schema`** — no `--stdout` flag,
   uniform with other output formats.
2. **`--stats` is gated to `-o json-schema`** — clap conflict with other
   formats; default off.
3. **Single `--enum-threshold` flag** — same value drives categorical
   detection in all renderings.
4. **`OutputFormat::JsonSchema`** is the enum variant; `json-schema` is
   the CLI surface name.
5. **MCP `profile` gains `format: "json-schema"` in the same PR**;
   MCP `schema` tool's `path`/`data` branch hard-errors in the same
   PR. (Type-key branch retention is card 0006's call.)
6. **Schema export verbosity is fixed** — only `x-finetype-label` and
   `x-finetype-pii` ship; all other `x-finetype-*` fields are dropped
   (PR #51 shipped this; this card inherits the contract).
7. **No regression in `profile`'s existing 5 output formats** — they
   keep their current shape and behaviour.
8. **Existing `golden_profile_*` tests stay green**; new tests added
   under a dedicated section.

### Success Criteria

- `finetype profile -f file.csv -o json-schema` emits a JSON Schema
  document to stdout for every fixture in `eval/datasets/csv/`.
- `finetype profile -f file.csv -o json-schema --stats` adds observed-
  data constraints matching today's `schema -f file.csv --stats` byte
  shape (modulo the trimmed extensions).
- `finetype profile -f file.csv -o json-schema | finetype validate -f
  file.csv --schema -` round-trips cleanly (depends on card 0005's
  `--schema -` stdin support; if not yet shipped, the test asserts
  output shape only).
- MCP `profile` tool with `format: "json-schema"` returns equivalent
  JSON Schema in its primary content slot.
- MCP `schema` tool's table-branch returns an error pointing at
  `profile` (and `taxonomy`, per card 0006).
- All `--help` text updated; no references to deprecated paths.
- 2-3 new `golden_profile_json_schema_*` tests added.

### Decisions Surfaced

- **D1 — Stdout-by-default for `-o json-schema`**: chose A over
  sidecar-default; consistent with sibling output formats.
- **D3 — `--stats` carry-over (gated)**: chose A over drop; preserves
  capability without leaking into other formats.
- **D4 — Single `--enum-threshold` flag**: chose A over per-verb split;
  matches users' intent.
- **D5 — Schema verb removal mechanics**: deferred to card 0006.
- **D6 — MCP `profile` gains `format` parameter**: chose A (same PR
  mirror); MCP `schema` table-branch dies.
- **D7 — `OutputFormat::JsonSchema` variant**: chose A; `json-schema`
  on CLI, kebab-case via clap derive.
- **D8 — Add `golden_profile_json_schema_*` tests**: chose A; cover
  round-trip, stats, enum-threshold.

### Implementation Notes

- `cmd_schema_table` (`crates/finetype-cli/src/main.rs:2699+`) is the
  table-mode JSON Schema emitter today. Lift its body into a shared
  helper module (per card 0006 D3 — CLI-internal helper, not yet
  `finetype-core`). Both `cmd_profile` (this card) and `cmd_taxonomy`
  (card 0006) call it.
- `OutputFormat` enum at `main.rs:476-483` adds `JsonSchema`; the
  match arms at `main.rs:4386-4757` (in `cmd_profile`) gain the new
  branch.
- `profile`'s clap variant gains `stats: bool` (default false); add
  clap conflict with other `-o` values via `requires`/`conflicts_with`
  on `output_format`. Or simpler: when `--stats` is set and
  `output_format != JsonSchema`, error in the dispatch arm with a
  helpful message.
- MCP `tools/profile.rs` gains `format` parameter; `tools/schema.rs`
  loses its `path`/`data` branch in the same PR.
- Migration text for the README example at line 82: `finetype schema
  data.csv --stdout > schema.json` becomes `finetype profile -f
  data.csv -o json-schema > schema.json`.

### Open Questions

- Whether the round-trip test (Q8 / first golden test) lands in this
  card or in the card 0005 spec — depends on whether `validate
  --schema -` (stdin) is in card 0005's scope. If not, this card
  asserts JSON Schema validity locally (e.g., via `serde_json` parse
  + spot-check assertions on shape).
- Edge case: `profile -o json-schema` on an Arrow-backed input
  (`-f file.arrow` or stdin Arrow). Not flagged in any memo; assume
  current `cmd_profile` Arrow path works through the new arm
  unchanged.
