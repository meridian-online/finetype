# Decision Pack — `profile -o json-schema`

**Card:** `orbit/cards/0003-tabular-data-profiling.yaml`
**Memos:** `orbit/memos/2026-04-27-schema-profile-overlap.md`,
`orbit/memos/2026-04-27-schema-export-verbosity.md`
**Baseline (already shipped, v0.6.19 PR #51):** `finetype --help`
exposes 7 verbs (`infer`, `profile`, `schema`, `validate`, `load`,
`mcp`, `taxonomy`). Both `schema` emitter codepaths in
`crates/finetype-cli/src/main.rs` already drop the derivable
`x-finetype-*` fields, retaining only `label` + `pii` (AC-6 of the
visibility-cleanup spec). The `schema` verb still exists; this card
folds its **table mode** (the path-sniffer at `main.rs:602-629`)
into `profile` as `-o json-schema`.

This pack scopes the decisions required to implement that fold.

The sibling card (taxonomy gains `-o json-schema`) is out of scope
here, but where decisions interact (shared emitter, `OutputFormat`
variant naming) the pack flags it explicitly.

---

## D1. Output destination semantics for `profile -o json-schema`

### Context

`finetype schema <file.csv>` today writes a sidecar
`<input>.schema.json` by default and emits to stdout when `--stdout`
is passed (`crates/finetype-cli/src/main.rs:3020-3035`). `finetype
profile -o json` writes to **stdout** with no sidecar and no
`--stdout` toggle (`main.rs:4574-4584`). The two verbs have opposite
defaults today; the fold has to pick one.

### Options

- **A. Stdout-by-default (match `profile` siblings).** `profile -o
  json-schema` writes to stdout, just like `-o json`, `-o csv`,
  `-o markdown`. No `--stdout` flag exists. Sidecar requires shell
  redirect (`> file.schema.json`).
- **B. Sidecar-by-default with `--stdout` toggle (match today's
  `schema <file.csv>`).** `profile -o json-schema` writes
  `<input>.schema.json` next to the input file. Add a `--stdout`
  flag (currently profile-specific) for the pipe case. All other
  `-o` formats stay stdout — `json-schema` becomes a special-case
  output mode.
- **C. Sidecar with explicit `--out <path>` flag, no implicit
  filename.** `profile -o json-schema` either writes to a
  user-named path or, in the absence of `--out`, to stdout. Splits
  the difference: nothing is implicit, but the sidecar workflow has
  ergonomic parity.

### Trade-offs

- **A (stdout-by-default).** Gains: every `-o` format has the same
  output destination — `profile -o X > file.X` is the universal
  shape. Migration from `schema -f file.csv --stdout > schema.json`
  is mechanical (`profile -f file.csv -o json-schema >
  schema.json`). Loses: the convenient default that the table-mode
  `schema` workflow has today (run-and-go produces a file named
  after the input). Users who relied on the implicit sidecar
  filename have to add a redirect — but that's a one-character fix
  and the error mode (file written to stdout, scrolling past) is
  loud.
- **B (sidecar-by-default).** Gains: byte-for-byte preserves the
  ergonomics of `schema <file.csv>`. Users typing the new command
  produce a file at the same path they were producing before.
  Loses: `-o json-schema` becomes the only `OutputFormat` whose
  destination differs from its siblings — every other `-o` value
  goes to stdout. That's a cross-cutting irregularity in a verb
  whose `--help` already reads
  `Output format (plain, json, csv, markdown, arrow)`. Each
  consumer of `OutputFormat` (the format string in `--help`, the
  match arms at `main.rs:4386-4757`, the MCP mirror) has to learn
  the special case.
- **C (`--out` flag).** Gains: makes the destination explicit, no
  surprise behaviours. Loses: a new flag for one output mode;
  doesn't compose cleanly with the taxonomy sibling card (taxonomy
  has no notion of "an input filename to derive a sidecar name
  from").

### Recommendation

**A — stdout-by-default, no `--stdout` flag.** The whole point of
the fold (memo `schema-profile-overlap.md` lines 154-187) is that
JSON Schema is "just another output shape" of the same inference
work. Treating it like every other `-o X` shape collapses one CLI
irregularity per fold, not one per fold per consumer. Migration is
trivial (`> file.schema.json` is one redirect symbol). The
sibling-card alignment (`taxonomy KEY -o json-schema` on stdout)
falls out for free.

The current `schema <file.csv> --stdout` users — verified to be the
form documented in `README.md:82` — already have the redirect form
in muscle memory. The implicit-sidecar form was a convenience for
"just give me a file" users; in v0.6.19's hard-removal posture
(visibility-cleanup spec constraint, line 16) the convenience trade
isn't worth the asymmetry.

---

## D2. Sidecar filename convention (only relevant if D1 picks B or C)

### Context

If a sidecar is produced (default in option B, opt-in in option C),
the filename has to be chosen. Today: `data.csv` →
`data.schema.json` via `input.with_file_name(<stem>.schema.json)`
at `main.rs:3026-3032`. Other tools in the ecosystem (e.g.,
profile-style runners) sometimes use `<input>.profile.json` or
`<input>.profile.schema.json` to disambiguate which producer wrote
the file.

### Options

- **A. Keep `<input>.schema.json`** (today's convention).
- **B. Adopt `<input>.profile.schema.json`** (encodes producer in
  the name).
- **C. Adopt `<input>.json-schema.json`** (encodes the format in
  the name).

### Trade-offs

- **A.** Continuity — files emitted by today's `schema <file.csv>`
  and tomorrow's `profile -o json-schema` end up at the same path,
  so committed schema files in users' repos don't relocate.
  Loses: nothing meaningful — the file was always JSON Schema, the
  word "schema" carries that.
- **B.** Self-describing filename for users with multiple
  profile-style outputs in the same directory. Loses: longer name,
  doesn't match any prior convention.
- **C.** Disambiguates from validation-engine "schema" usage (e.g.
  database schemas). Loses: even longer, awkward double-`.json`,
  no precedent.

### Recommendation

**A — keep `<input>.schema.json`.** This decision only matters if
D1 picks B or C; if D1 picks A (stdout-by-default), the sidecar
filename is the user's choice via redirect. Continuity wins. Any
existing committed schema files keep their paths.

---

## D3. `--stats` carry-over

### Context

`schema <file.csv> --stats` adds observed-data constraints to each
property: `minLength`/`maxLength` for strings, `minimum`/`maximum`
for numerics, `enum` (when cardinality ≤ `--enum-threshold`),
`x-finetype-null-rate`, `x-finetype-cardinality`
(`main.rs:2928-2987`). `profile`'s other output formats today don't
emit any of those fields.

### Options

- **A. Carry `--stats` over to `profile -o json-schema` only.** Add
  a `stats: bool` flag to `Profile`'s clap variant; when set and
  `output == JsonSchema`, the emitter computes and writes the same
  fields as today. With other output formats, `--stats` errors via
  a clap conflict.
- **B. Drop `--stats` for v0.6.20.** The schema is the type
  contract; observed data lives in profile output. Users who want
  observed-data fields use `profile -o json` and merge.
- **C. Promote `--stats` to a profile-wide flag (no output-format
  gating).** All output formats gain the observed-data fields where
  meaningful: `json` learns the same set of keys, `csv` adds them
  as columns, etc.

### Trade-offs

- **A (gated carry-over).** Gains: behaviour-preserving. Existing
  `schema <file.csv> --stats` users get a one-for-one migration to
  `profile -f file.csv -o json-schema --stats`. The flag's
  semantics ("attach observed-data constraints to JSON Schema
  output") stay the same. Loses: `--stats` becomes one of two
  flags on `profile` whose effect depends on `-o` (the other being
  `--enum-threshold`, see D4); discoverability is mediocre. A clap
  `requires`/`conflicts_with` constraint is needed to keep
  `--stats` from no-op'ing under `-o json`.
- **B (drop `--stats`).** Gains: smallest diff, smallest surface,
  fewest cross-flag interactions. The schema-export-verbosity memo
  (`2026-04-27-schema-export-verbosity.md`) already established
  the principle that schemas are contracts and provenance lives
  elsewhere — observed-data constraints are arguably provenance,
  not contract. Loses: hard removal of an existing capability.
  Anyone running `schema <file.csv> --stats` today loses the only
  CLI path to a schema with observed-data constraints; they'd have
  to merge `profile -o json` output into a JSON Schema by hand or
  build it themselves.
- **C (promote profile-wide).** Gains: composes cleanly — no
  output-format gating. Loses: significantly larger diff (rendering
  for plain/csv/markdown/arrow has to be designed); leaks
  observed-data fields into formats whose audience didn't ask for
  them.

### Recommendation

**A — carry `--stats` over to `profile -o json-schema`, gated.**
Behaviour preservation is the strongest signal here: the v0.6.19
visibility-cleanup spec (line 16, hard-removal posture) tightened
the surface but explicitly preserved capabilities — this fold
should follow the same pattern. `--stats` is genuinely optional
(default off) so it's not paying the surface-area tax for
non-users. The clap `requires` pattern is well-trod (validate's
`--db`/`--table` pair — `main.rs:307-319` — already uses it).

A future memo could reopen B if the observed-data-as-provenance
argument from the verbosity memo gets traction; for now, capture
that as a follow-up not a blocker.

---

## D4. `--enum-threshold` interaction

### Context

`profile` already has `--enum-threshold` (default 50,
`main.rs:357-359`); it controls when ENUM is emitted in plain/json
output and is wired through `cmd_profile`. `schema <file.csv>`
also has `--enum-threshold` (default 50, `main.rs:219-221`); it
controls when an `enum` keyword is added under `--stats`
(`main.rs:2983-2987`).

### Options

- **A. Reuse profile's existing `--enum-threshold` for the JSON
  Schema emitter.** Same flag, same default, applied uniformly:
  controls categorical detection in plain/json/markdown/csv
  rendering AND the `enum` keyword in `-o json-schema --stats`.
- **B. Keep them as separate flags.** `--enum-threshold` for
  profile rendering, a different flag (e.g.
  `--schema-enum-threshold`) for the JSON Schema `enum` keyword.

### Trade-offs

- **A.** Gains: one flag, one default — matches users' mental
  model ("how many distinct values before I treat this as
  categorical?"). Trivial to implement: `cmd_profile` already has
  the value, the JsonSchema arm just reads it. Loses: a user who
  wants `enum` in JSON Schema at a different threshold than ENUM
  in plain output is out of luck — but that's a contrived case and
  arguably the wrong one (the answer is "they should match"). The
  ENUM threshold for plain output is purely cosmetic; the schema
  one is wire-format. Conflating the two is semantically tight.
- **B.** Gains: independent control. Loses: more flags, two
  defaults, redundant in practice.

### Recommendation

**A — single `--enum-threshold` flag, applied to both renderings.**
Lines up with users' intent (one threshold for "is this column
categorical?"). The implementation is a no-op — `cmd_profile`
already has `enum_threshold: usize` in its signature
(`main.rs:4078`); the new JsonSchema arm just consumes it.

---

## D5. Behaviour of `finetype schema` in the v0.6.20 release

### Context

Visibility-cleanup spec constraint (line 20) says: "Schema verb
consolidation (type-mode → taxonomy, table-mode → profile) is out
of scope. The schema verb still exists in v0.6.19; only its export
verbosity changes." So at the start of this card's work, `finetype
schema` is still public. The schema-profile-overlap memo's revised
migration plan (lines 207-214) calls for "Deprecate `finetype
schema` (both modes) for one release; emit a warning pointing to
the replacement verb. Remove `schema` verb entirely in the
following release."

But the v0.6.19 release notes already use **hard-removal** posture
(visibility-cleanup spec, line 16) — explicit clap errors for
removed surfaces, no parallel-maintenance window. So there's a
posture choice for the schema verb in v0.6.20.

### Options

- **A. Hard-remove the `schema` verb in v0.6.20.** Same release as
  the `profile -o json-schema` and `taxonomy -o json-schema`
  additions. `finetype schema KEY` and `finetype schema
  <file.csv>` both error via clap's unknown-subcommand handler
  pointing at the new verbs. CHANGELOG carries the migration map.
- **B. Soft-deprecate the verb for one release.** `schema` keeps
  working but prints a warning to stderr at the top of each
  invocation (`schema <file.csv>` → "use profile -f <file> -o
  json-schema"; `schema KEY` → "use taxonomy KEY -o json-schema").
  Removed in v0.6.21.
- **C. Hard-error with migration message in v0.6.20, no
  silent-warn intermediate.** Variant of A: `schema` exists in
  the clap subcommand enum but its handler immediately prints the
  migration map to stderr and exits non-zero. Invokers see
  guidance, not a generic "unknown command" message.

### Trade-offs

- **A (clean hard-remove).** Gains: matches v0.6.19's hard-removal
  posture (visibility-cleanup constraint, line 16); the same
  trade-off Hugh accepted there ("scripts pinning the old shape
  get an explicit clap error… recoverable, the migration is
  one-line"). Smallest diff. Loses: clap's stock unknown-subcommand
  message ("error: unrecognized subcommand 'schema'") is generic;
  doesn't tell the user where to go. They have to consult the
  CHANGELOG.
- **B (soft-deprecate).** Gains: one release of overlap means
  scripts have a window to migrate before failing. Most ergonomic
  for users mid-migration. Loses: parallel-maintenance window —
  two codepaths (the old `schema` handler, the new
  `profile -o json-schema` and `taxonomy -o json-schema`) both
  alive in the codebase, both subject to bug reports, both subject
  to drift from each other. Same tradeoff Hugh rejected at the
  v0.6.19 boundary.
- **C (informative hard-error).** Gains: hard removal *and* the
  user gets pointed at the replacement at point of failure. The
  schema verb's `Commands::Schema` variant becomes a thin error
  handler — no maintenance burden of keeping both codepaths alive.
  Loses: marginally larger diff than A (a handler instead of a
  deletion); confusion about whether the verb "exists" (it parses,
  but only to error).

### Recommendation

**C — hard-error with migration message.** Gets the
hard-removal-posture consistency with v0.6.19 (no parallel
maintenance, no drift), but doesn't strand users in front of a
generic clap error message. Implementation is a 10-20 line shim:
keep the `Commands::Schema` variant, replace its dispatch arm
with a printer that detects whether `type_key` looks like a file
path (the same path-sniffer at `main.rs:604-615` runs for free)
and emits one of two migration messages, then exits with code 2.

The MCP `schema` tool follows the same pattern (D6 below).

A future v0.6.21 release can drop the shim entirely, by which
point everyone has migrated.

---

## D6. MCP mirror — `profile` tool gains `format` parameter

### Context

The MCP `profile` tool (`crates/finetype-mcp/src/tools/profile.rs`)
currently emits a fixed JSON shape (per-column array) plus a
markdown summary. The MCP `schema` tool
(`crates/finetype-mcp/src/tools/schema.rs:330-357`) handles **both**
type-mode (via `type_key`) and table-mode (via `path`/`data`) with
the same dispatcher. Visibility-cleanup constraint (line 19): "MCP's
`schema` tool stays verbose for v0.6.19 — it dies entirely in
pipeline-reshape (v0.6.20) when `taxonomy` and `profile` absorb
json-schema output."

The MCP surface mirrors the CLI surface; this card has to keep them
in lockstep.

### Options

- **A. Add `format: "json-schema"` parameter to MCP `profile`
  tool; remove MCP `schema` tool's table-mode branch in same PR.**
  MCP `profile` gains an enum-like parameter (default `"json"`);
  when set to `"json-schema"`, the tool runs the existing
  classification path and projects results through the JSON Schema
  emitter (the same one used by the CLI, ideally extracted to a
  shared helper). MCP `schema` tool's `path`/`data` branch becomes
  a hard error pointing at `profile`. Type-key branch removal is
  the sibling card's responsibility (taxonomy MCP tool).
- **B. Add separate `output_format` parameter to MCP `profile`,
  retain MCP `schema` table-mode branch.** Don't touch MCP
  `schema` until v0.6.21. Migration path exists but the old path
  also exists.
- **C. Keep the MCP `profile` tool single-shape (no new
  parameter); don't surface JSON Schema output in MCP at all.**
  MCP `schema` tool's table-mode branch survives, becomes the
  only MCP path to a table JSON Schema. Diverges CLI from MCP.

### Trade-offs

- **A (mirror in same PR).** Gains: CLI and MCP move together,
  visibility-cleanup constraint satisfied (MCP `schema` tool's
  table-mode branch dies as promised in v0.6.20). One JSON Schema
  emitter shared between two surfaces — no drift. Loses: bigger
  diff in this card; the JSON Schema emitter has to be extracted
  out of `cmd_schema_table` (`main.rs:2699-3038`) into a shared
  function so both `cmd_profile` and the MCP `profile` handler
  call it. That extraction is the right move regardless and pays
  for itself the moment the taxonomy sibling card lands.
- **B (parallel-maintain).** Gains: smaller diff. Loses: violates
  the v0.6.19 promise that MCP schema dies in v0.6.20; defers MCP
  cleanup to v0.6.21+; two codepaths alive in MCP for two
  releases.
- **C (CLI-only, MCP unchanged).** Gains: minimum scope. Loses:
  CLI/MCP divergence — one surface gets JSON Schema via
  `profile`, the other still via `schema`. Bad UX, harder docs.

### Recommendation

**A — `format` parameter on MCP `profile`, MCP `schema`
table-mode branch removed in same PR.** Mirroring is the load-
bearing principle: visibility-cleanup spec line 19 already promises
this for v0.6.20. The shared-emitter extraction is good engineering
(eliminates duplicate JSON Schema construction across
`cmd_schema_table`, `cmd_schema` type-mode, MCP `schema`'s
`build_json_schema` and `handle_file`), and the test surface
constrains drift.

Parameter naming: `format` (matches CLI `-o`) over `output_format`
(verbose). Values: `"json"` (default — current behaviour),
`"json-schema"`.

The MCP `schema` tool's type-key branch is retained for now (the
sibling card folds it into MCP `taxonomy`); but its `path`/`data`
branch (`schema.rs:336-348`) is replaced with an error pointing at
`profile`. Same migration-message pattern as D5's option C.

---

## D7. `OutputFormat` enum variant naming

### Context

The clap `OutputFormat` enum at `main.rs:476-483` currently has
`Plain`, `Json`, `Csv`, `Markdown`, `Arrow`. Adding the new variant
needs a name that reads well in `--help` and on the command line.
Sibling card adds the same variant to a different verb's clap parse;
the names should match.

### Options

- **A. `JsonSchema` (kebab-cased on CLI as `json-schema`).** Adds
  one variant to the existing enum.
- **B. `Schema`.** Shorter, but ambiguous — "schema" already names
  the verb being deprecated, and at the wire-format level "schema"
  is overloaded (DuckDB schema vs JSON Schema vs taxonomy schema).
- **C. Split `OutputFormat` per verb (profile gets its own enum
  with a JSON Schema variant).** Heavier refactor.

### Trade-offs

- **A.** Gains: explicit, no overload, mirrors the JSON Schema
  spec name. Reads naturally as `-o json-schema`. clap's
  `value_enum` derive handles the kebab-case automatically. Loses:
  marginally longer than `schema`.
- **B.** Gains: shortest. Loses: ambiguous; collides conceptually
  with the deprecated verb; sets up for future overload.
- **C.** Gains: lets each verb have its own format set (e.g.
  taxonomy could refuse `arrow`). Loses: bigger refactor with no
  clear payoff in this card; existing format-mismatch behaviour
  (e.g. `infer -o markdown`) is just "format ignored" today.

### Recommendation

**A — `JsonSchema` enum variant, `json-schema` on the CLI.**
Trivial, explicit, no overload, mirrors the spec it implements.
Sibling card uses the same variant on its own (or shared) enum.

---

## D8. Test surface migration

### Context

`crates/finetype-cli/tests/cli_golden.rs` has two schema-test
helpers (`run_schema_json` at line 142) and two table-mode-using
tests visible at lines 638 (`golden_schema_email`) and 740
(`golden_schema_iso_date`). Both **type-mode** — they call
`run_schema_json("identity.person.email")` etc. Searching the file
for table-mode `schema <file>` golden tests finds none — table
mode appears only as a code-path in `cmd_schema_table`, with no
existing golden coverage.

### Options

- **A. Add `golden_profile_json_schema_*` tests modelled on the
  existing `golden_profile_*` tests.** Cover:
  (a) round-trip — `profile -o json-schema` output validates
  against itself via `validate <csv> <schema.json>`,
  (b) `--stats` produces observed-data fields,
  (c) `--enum-threshold` controls the `enum` keyword. Migrate the
  README example `schema data.csv --stdout > schema.json` to
  `profile -f data.csv -o json-schema > schema.json`.
- **B. Don't add new golden tests; rely on existing
  `golden_schema_*` tests (still pointing at `schema KEY`) for
  type-mode coverage and unit tests for the new JSON Schema arm
  in `cmd_profile`.** The two type-mode tests (`golden_schema_email`,
  `golden_schema_iso_date`) only fail if the type-mode emitter
  changes; they don't cover the table fold.
- **C. Migrate existing `golden_schema_*` tests onto
  `profile -o json-schema` and delete the type-mode helper.** Bad
  fit — those tests cover type-mode (taxonomy export); only the
  sibling card moves them.

### Trade-offs

- **A.** Gains: coverage where it's needed — the new code path,
  plus the round-trip-with-validate property the card implies
  (scenario 3 in the card: "FineType emits a table-level JSON
  Schema… replacing the earlier table-mode invocation of finetype
  schema"). Loses: 2-4 new test functions (the existing
  `golden_profile_*` tests are relatively cheap to copy).
- **B.** Gains: minimal test churn. Loses: zero coverage of the
  new code path's correctness — the card's primary scenario goes
  unverified at the integration level.
- **C.** Wrong cut — type-mode is the sibling card's
  responsibility; this card doesn't touch type-mode tests except
  to keep them passing.

### Recommendation

**A — add two-three `golden_profile_json_schema_*` tests.**
Coverage at the seams that matter: the round-trip, `--stats`
behaviour, `--enum-threshold` behaviour. The fixture CSV files
already exist (the existing `golden_profile_*` tests
(`cli_golden.rs` references `ecommerce_orders`, `people_directory`,
`titanic`) so reusing them is mostly mechanical.

Existing `golden_schema_email` and `golden_schema_iso_date` tests
(`cli_golden.rs:638, 740`) stay as-is — they cover type-mode,
which is the sibling card's concern. Per D5, they will need to
either migrate to `taxonomy KEY -o json-schema` (sibling card's
job) or accept the hard-error (this card's `schema` shim, in which
case they need to be deleted by the sibling card).

The README example at `README.md:82` (`finetype schema data.csv
--stdout > schema.json`) is updated to `finetype profile -f
data.csv -o json-schema > schema.json` in the same PR. Same for
the `## CLI commands` table in `CLAUDE.md:191` (the schema row).

---

## Summary table

```
| ID | Decision                                | Recommendation                                |
|----|-----------------------------------------|-----------------------------------------------|
| D1 | Output destination                      | A — stdout-by-default, no --stdout flag       |
| D2 | Sidecar filename (only if D1=B/C)       | A — keep <input>.schema.json                  |
| D3 | --stats carry-over                      | A — gated, only with -o json-schema           |
| D4 | --enum-threshold interaction            | A — single flag, applied to both renderings  |
| D5 | finetype schema verb in v0.6.20         | C — hard-error with migration message         |
| D6 | MCP profile tool gains format param     | A — same PR, MCP schema table branch dies    |
| D7 | OutputFormat variant naming             | A — JsonSchema enum, json-schema on CLI       |
| D8 | Test surface migration                  | A — add golden_profile_json_schema_* tests    |
```

## Decisions out of scope for this card

- **Type-mode JSON Schema export under `taxonomy` verb.** Sibling
  card. This pack flags interactions in D6 and D7 but does not
  decide them.
- **Schema-export verbosity.** Already shipped in v0.6.19 PR #51
  (visibility-cleanup AC-6). The lean shape (`x-finetype-label` +
  `x-finetype-pii` only, all else dropped) is the contract this
  card inherits — no re-litigation.
- **Whether to keep the `schema` verb as a soft-deprecation in
  v0.6.21+.** D5 picks hard-error in v0.6.20; the shim's lifetime
  is a release-management call that can be made at v0.6.21
  planning, not a design decision for this card.
