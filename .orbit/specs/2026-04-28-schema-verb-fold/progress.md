# Progress — schema verb fold (card 0006)

Spec: `.orbit/specs/2026-04-28-schema-verb-fold/spec.yaml` (v1.1, 11 ACs)

## Acceptance criteria

- [x] ac-01 — `emit_type_schema` in `crates/finetype-mcp/src/json_schema.rs` (label + pii extensions, $schema/$id/title/description/validation/examples)
- [x] ac-02 — `Commands::Taxonomy` gains positional `type_key: Option<String>` + glob + edit-distance suggestions
- [x] ac-03 — `cmd_taxonomy` `OutputFormat::JsonSchema` arm — always-array, unconditional pretty-print, unknown-key contract preserved
- [x] ac-04 — Delete `Commands::Schema`, `cmd_schema`, `cmd_schema_table`, dispatch arm, `-f, --file` schema flag, `build_json_schema`; `clippy -D dead_code` clean
- [x] ac-05 — Rename `golden_schema_*` → `golden_taxonomy_json_schema_*`, add `x-finetype-label` presence assertion
- [x] ac-06 — README/CLAUDE.md/.claude/skills migration; "7 → 6 public commands"
- [x] ac-07 — CHANGELOG entry with verbatim migration map + Changed sub-bullet for `x-finetype-label`
- [x] ac-08 — MADR 0070 created
- [x] ac-09 — MADR 0031 frontmatter → `superseded by 0070`
- [x] ac-10 — MCP audit follow-up comment markers in `lib.rs` + `tools/schema.rs`
- [x] ac-11 — `make ci` passes; clippy on touched crates clean (PR #53 baseline)

## Cycle-2 LOW finding folds

- ac-04: spec line refs (2565/2699/2642/595) drift from current main HEAD (2601/2735/2678/608); function names + grep verification keep this non-load-bearing.
- ac-06: CLAUDE.md MCP row gets the inline "MCP audit follow-up in v0.6.20" note; ac-10 carries the symmetric grep.

## Implementation order

1. ac-01 — port emitter into `finetype-mcp/src/json_schema.rs` with unit test
2. ac-02 — add positional `type_key` to `Commands::Taxonomy` clap variant + thread through `cmd_taxonomy`
3. ac-03 — add `OutputFormat::JsonSchema` arm to `cmd_taxonomy`
4. ac-04 — delete `Commands::Schema`, `cmd_schema`, `cmd_schema_table`, dispatch arm, `build_json_schema`
5. ac-05 — rename golden tests, update body
6. ac-08 + ac-09 — MADR 0070 + supersede 0031
7. ac-10 — MCP follow-up comments
8. ac-06 — README/CLAUDE.md/.claude/skills migration
9. ac-07 — CHANGELOG
10. ac-11 — `make ci` gate
