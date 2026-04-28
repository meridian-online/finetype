# Design: `schema` verb fold

**Date:** 2026-04-28
**Interviewer:** Nightingale (lead, with author Hugh)
**Card:** `orbit/cards/0006-command-line-interface.yaml`
**Rally:** `orbit/specs/2026-04-28-v0619-cli-consolidation-rally/`
**Sibling cards:** 0003 (profile -o json-schema), 0005 (validate absorbs load)
**Target release:** v0.6.19

---

## Context

**Card:** *Command-line interface* — 6 scenarios, goal: "Four public
verbs (infer | profile | validate | mcp + taxonomy with json-schema
output) composing into pipelines; maintainer affordances hidden;
surface mirrors MCP."

**Prior specs:**
- `2026-04-27-cli-visibility-cleanup/` — first half of v0.6.19 (shipped,
  PR #51): hid 6 internal commands, removed `--model`, `--sharp-only`,
  `eval-gittables`, trimmed JSON Schema export to label + pii.

**Gap:** The v0.6.19 first PR left the `schema` verb in place, with its
two modes intact. This card retires the verb entirely. Type-mode
(`finetype schema KEY`) folds into `taxonomy KEY -o json-schema`;
table-mode (`finetype schema <file.csv>`) folds into card 0003's
`profile -f file.csv -o json-schema`. The deletion + the type-mode
replacement live here.

**Source memos:**
- `orbit/memos/2026-04-27-schema-profile-overlap.md` (revised "Both
  modes fold" section)
- `orbit/memos/2026-04-27-schema-cli-flag-collision.md` (superseded;
  becomes moot with the verb gone)
- `orbit/memos/2026-04-27-mcp-surface-audit.md` (governs MCP timing)

---

## Q&A

### Q1: Removal cadence — hard remove in v0.6.19, or soft-deprecate?

**Q:** The original memos drafted a one-release deprecation cycle
("warn in v0.6.19, remove in v0.6.20"). PR #51 ratified the
hard-removal posture for the v0.6.19 line. Which wins?

**A:** Hard removal in v0.6.19. Same PR removes:
- `Commands::Schema` variant
- `cmd_schema` function (`crates/finetype-cli/src/main.rs:2565-2639`)
- `cmd_schema_table` function (`main.rs:2699+`)
- Path-sniffer dispatch arm (`main.rs:595-629`)
- `-f, --file` flag bound to schema's `file: PathBuf` arg (`main.rs:204-205`)

Both replacements ship in the same release (0003: `profile -o
json-schema`; this card: `taxonomy -o json-schema`), so the migration
path is one-line per script. Users get clap's "unrecognized subcommand"
error; CHANGELOG carries the migration map. No dispatch shim, no
warning suppression, no parallel-maintenance window.

### Q2: `taxonomy KEY -o json-schema` — array or object for single match?

**Q:** Today `schema email` returns a single object, `schema
"datetime.date.*"` returns an array. After fold, what shape should
`taxonomy email -o json-schema` return?

**A:** Always an array. Consistency with the rest of `cmd_taxonomy`'s
output formats — `Plain`, `Json`, `Csv`, `Markdown`, `Arrow` all
iterate over the matched set. `JsonSchema` matches that pattern. The
single-key migration cost (`finetype schema email | jq .pattern`
becomes `finetype taxonomy email -o json-schema | jq '.[0].pattern'`)
is documented in the CHANGELOG; output shape doesn't depend on
cardinality.

### Q3: Where does the JSON Schema emitter live?

**Q:** Today `build_json_schema` is private inside `main.rs`. After
fold, both `cmd_taxonomy` (this card) and `cmd_profile` (card 0003)
need it. MCP also has its own copy. Lift to `finetype-core`, lift to
a CLI-internal helper module, or duplicate inline?

**A:** Lift to a CLI-internal helper module
(`crates/finetype-cli/src/json_schema.rs` or similar). No
`finetype-core` public API expansion in this rally — MCP is deferred
(Q4); the only consumers are CLI `cmd_taxonomy` and CLI `cmd_profile`.
When the MCP audit ships in v0.6.20 and surfaces a third consumer,
either the helper moves up to `finetype-core` then, or — more likely
per the MCP audit memo — the MCP `schema` tool dies entirely and MCP
`taxonomy`/`profile` tools call equivalent emitter logic via a small
port. Defer that decision until there's a concrete second consumer.

### Q4: MCP `schema` tool disposition

**Q:** Drop the MCP `schema` tool now (mirror CLI), hide it, or stub
it? The visibility-cleanup spec deferred this explicitly.

**A:** Defer to v0.6.20 MCP audit. Visibility-cleanup constraint line
19 was deliberate: MCP audit comes after the CLI shape is settled, so
MCP changes are mechanical mirrors. Card 0003 D6 specifies that the
MCP `schema` tool's table-branch hard-errors when card 0003 ships
(consistency with CLI's table-mode disappearance). The type-key
branch survives v0.6.19 untouched — MCP tool count stays at 8;
description text updates noted in CLAUDE.md as the mirror gap.

The CLI/MCP-mirror invariant (card 0006 scenario "Public surface
mirrors the MCP server") is broken for one release in a controlled
way, documented in this spec, with an explicit follow-up reference
to the MCP audit spec. Same posture the visibility-cleanup spec took
for the schema verbosity reduction itself.

### Q5: Migration surface — which docs/tests/examples?

**Q:** The verb removal breaks every example. Which loci ship in the
same PR, and which can defer?

**A:** Same-PR migration of code + docs + tests:

- `README.md:77` — `finetype schema "datetime.date.*" --pretty` →
  `finetype taxonomy "datetime.date.*" -o json-schema --pretty`
- `README.md:82` — `finetype schema data.csv --stdout > schema.json` →
  `finetype profile -f data.csv -o json-schema > schema.json`
- `CLAUDE.md:208` — public-vs-internal command table row removed
- `CLAUDE.md:165, 211` — MCP tools table mention of `schema` retained
  for v0.6.19 (per Q4); add a "MCP audit follow-up" note
- `.claude/skills/finetype-cli/SKILL.md:40-65, 244, 251` — migrated
- `.claude/skills/finetype-pipeline/SKILL.md:57, 140` — migrated
- `crates/finetype-cli/tests/cli_golden.rs:143-159` — `run_schema_json`
  helper renamed to `run_taxonomy_json_schema` (or similar). Tests:
  - `golden_schema_email` → `golden_taxonomy_json_schema_email`
  - `golden_schema_iso_date` → `golden_taxonomy_json_schema_iso_date`
- `CHANGELOG.md` — v0.6.19 entry with explicit `grep schema` migration
  map
- `orbit/decisions/0031-table-level-schema-via-finetype-schema.md` —
  frontmatter `status: superseded by 0070`

Out of scope (not user-facing): `orbit/specs/*/` historical references,
historical CHANGELOG entries.

Per Q2 (always-array), the two migrated tests gain `.as_array()
.unwrap()[0]` projection on the JSON parse; existing assertions
(presence of `pattern`, `x-finetype-pii`; absence of derivable
`x-finetype-*` extensions) stay unchanged — verbosity reduction
shipped in PR #51.

### Q6: Decision-register entry

**Q:** Visibility-cleanup spec line 374-375 promised a MADR for "schema
verb folds entirely." Where does it land?

**A:** Write **MADR 0070** in the same PR as the code change. Title:
"Schema verb folds entirely — type-mode → taxonomy, table-mode →
profile." Status `accepted`. Reference 0031 as superseded. Flip 0031's
frontmatter to `status: superseded by 0070`.

The MADR can be written from the memos almost verbatim
(`schema-profile-overlap` lines 154-204 supplies Considered Options +
Decision Outcome + Consequences). Cross-link from this card's spec,
the rally manifest, and the two sibling spec dirs.

---

## Summary

### Goal

Retire `finetype schema` as a public verb. Add `OutputFormat::JsonSchema`
to `finetype taxonomy` so `taxonomy KEY -o json-schema` replaces
type-mode `schema KEY`. Land MADR 0070 capturing the architectural
move. Same PR migrates README, CLAUDE.md, skills docs, and golden
tests. MCP follows in v0.6.20 audit.

### Constraints

1. **Hard removal in v0.6.19** — no soft deprecation, no dispatch shim.
   Clap's unknown-subcommand error is the migration signal.
2. **`taxonomy -o json-schema` always returns a JSON array** — no
   cardinality-dependent shape.
3. **Shared JSON Schema emitter is a CLI-internal helper module**,
   consumed by `cmd_taxonomy` (this card) and `cmd_profile` (card 0003).
   No `finetype-core` API expansion in v0.6.19.
4. **MCP `schema` tool retained for v0.6.19** with a documented
   follow-up to the v0.6.20 audit. (Card 0003 D6: MCP `schema` table-
   branch dies in card 0003's PR; MCP `schema` type-branch lives until
   the audit.)
5. **Same-PR migration of code + docs + tests + CHANGELOG** — README
   examples must work against the binary in the released artefact.
6. **MADR 0070 written and 0031 superseded** in the same PR.
7. **Sibling-card alignment**: `OutputFormat::JsonSchema` enum variant
   shared with card 0003 (same `OutputFormat` enum, same kebab-case
   `json-schema` CLI surface). The lift-to-helper happens in card 0003's
   PR or this one — whichever lands first; the other consumes it.
8. **`-f, --file` flag at `main.rs:204-205`** (the schema-cli-flag-
   collision memo's offender) goes away with the verb — confirmed moot.

### Success Criteria

- `finetype --help` lists 6 public commands: `infer`, `profile`,
  `validate`, `mcp`, `taxonomy`, plus the existing `infer`-related
  variants. `schema` not present.
- `finetype schema KEY` and `finetype schema <file.csv>` both error
  with clap's unrecognized-subcommand message.
- `finetype taxonomy email -o json-schema` returns `[{ ...JSON Schema
  for identity.person.email... }]` — single-element array, lean
  extensions (label + pii only).
- `finetype taxonomy "datetime.date.*" -o json-schema` returns an
  array of all matching JSON Schema documents.
- README, CLAUDE.md, skill files, and CHANGELOG all reference the new
  verbs only; `grep "finetype schema" .` (excluding orbit/) returns no
  hits.
- Golden tests `golden_taxonomy_json_schema_email` and
  `golden_taxonomy_json_schema_iso_date` pass; old `golden_schema_*`
  tests deleted.
- MADR 0070 exists; MADR 0031's frontmatter shows
  `status: superseded by 0070`.
- MCP `schema` tool still works in MCP listings (per Q4); MCP follow-
  up referenced in this spec.

### Decisions Surfaced

- **D1 — Hard removal in v0.6.19**: chose A over soft-deprecate;
  matches PR #51's posture.
- **D2 — Always-array `taxonomy -o json-schema`**: chose A over
  cardinality-dependent shape; consistent with rest of `taxonomy`
  output formats.
- **D3 — CLI-internal helper module**: chose B over `finetype-core`
  lift; defer the cross-crate move until MCP needs it.
- **D4 — MCP `schema` tool deferred**: chose A; visibility-cleanup
  carve-out preserved for one more release.
- **D5 — Same-PR migration**: chose A over deferred docs; README
  must work in the release.
- **D6 — MADR 0070 in the same PR**: chose A over deferred MADR;
  Mission value "Decisions captured, not forgotten."

### Implementation Notes

- New helper module: `crates/finetype-cli/src/json_schema.rs`. Two
  public functions:
  - `build_type_json_schema(label: &str, …) -> serde_json::Value` —
    extracted from `cmd_schema`'s body
  - `build_table_json_schema(headers: &[String], extensions: &[…], …)
    -> serde_json::Value` — extracted from `cmd_schema_table`'s body
  Both shared with card 0003. The first stable lands the helper;
  the second consumes it.
- `OutputFormat` enum at `main.rs:476-483` gains `JsonSchema`. Both
  `cmd_taxonomy` and `cmd_profile` match on it.
- `cmd_taxonomy` at `main.rs:2400-2488` gains a `JsonSchema` arm:
  iterate over matched type keys, call `build_type_json_schema` per
  key, collect into `Vec<Value>`, serialize with optional pretty.
- `Commands::Taxonomy` clap variant gains `output: OutputFormat` if
  not already present (verify in spec authoring).
- Deletions: `Commands::Schema` (`main.rs:225-265` region), dispatch
  arm at `main.rs:595-629`, `cmd_schema` at `main.rs:2565-2639`,
  `cmd_schema_table` at `main.rs:2699+` (after extracting body to
  helper module). The `-f, --file` flag at `main.rs:204-205` (bound
  to schema's `file: PathBuf` arg).
- MADR 0070 file: `orbit/decisions/0070-schema-verb-folds-entirely.md`.
  Status `accepted`. Title "Schema verb folds entirely — type-mode →
  taxonomy, table-mode → profile."
- Decision 0031 frontmatter flip: `status: accepted` → `status:
  superseded by 0070`. `date-modified: 2026-04-28`.
- Test renames: keep assertion bodies; add `.as_array().unwrap()[0]`
  in front of the JSON shape inspection.

### Open Questions

- **Helper-module home**: `crates/finetype-cli/src/json_schema.rs` vs
  `crates/finetype-cli/src/schema_emitter.rs`. Cosmetic; pick during
  spec authoring.
- **Whether card 0003 or this card lands the helper-module commit
  first.** Probably card 0003 (the table emitter is the bigger lift);
  this card consumes it. Coordinate during the implementation phase.
- **CHANGELOG section split**: one combined "v0.6.19 — schema verb
  retired" or separate entries per sibling card? Single combined
  entry recommended; clearer for users.
