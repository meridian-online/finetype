# Implementation progress — profile json-schema output (card 0003, v0.6.19)

Spec: `spec.yaml` v1.0 (11 ACs, 10 constraints) — review-spec APPROVE on 2026-04-28.

## Acceptance Criteria

- [x] ac-01 — `OutputFormat::JsonSchema` variant added; `-o json-schema` callable.
- [x] ac-02 — `crates/finetype-mcp/src/json_schema.rs` helper module with `emit_table_schema()`. *(Home moved from CLI to MCP — CLI already depends on MCP, so the helper is reachable from both call sites without violating constraint #9 against finetype-core promotion.)*
- [x] ac-03 — `cmd_profile` `OutputFormat::JsonSchema` branch routes through helper, writes pretty-JSON to stdout.
- [x] ac-04 — `--stats` flag on `Profile` clap; conflict with non-`json-schema` `-o` raises clap-style error via `CommandFactory::command().error(ArgumentConflict, …).exit()`.
- [x] ac-05 — `--stats` adds `minLength`/`maxLength`/`minimum`/`maximum`/`enum` + `x-finetype-null-rate`/`x-finetype-cardinality`. (Type-validation contracts may also inject these keywords independently — that's by design and documented in `json_schema.rs` module docs.)
- [x] ac-06 — MCP `profile` gains `format: "json" | "json-schema"` parameter, plus `stats` and `enum_threshold` (default 50). Routes through the same helper as the CLI.
- [x] ac-07 — MCP `schema` `path`/`data` branch hard-errors with the verbatim migration string `"Table-mode schema export was folded into profile in v0.6.19. Use profile with format: \"json-schema\" instead."`.
- [x] ac-08 — three new `golden_profile_json_schema_*` tests under `cli_golden.rs`, all pass under `--ignored`.
- [x] ac-09 — README example migrated to `finetype profile -f data.csv -o json-schema > schema.json`. Help text inherited via `Profile` clap derive (`-o`/`--output` already documents the new variant).
- [x] ac-10 — round-trip parity: degraded structural-shape assertions land in `golden_profile_json_schema_people_directory` (top-level `$schema`/`$id`/`type:object`/`properties`; `x-finetype-label`/`x-finetype-pii` present; dropped extensions absent). Card 0005 will replace this with `validate --schema -` once stdin support ships.
- [x] ac-11 — `make ci` passes (fmt, clippy, test, taxonomy check). 240/240 taxonomy definitions pass; 12000/12000 generator samples pass.

## Implementation order (executed)

1. ✅ Helper module — created at `crates/finetype-mcp/src/json_schema.rs` with module-level docs covering the verbosity contract and `--stats` semantics.
2. ✅ CLI enum + Profile clap variant — `OutputFormat::JsonSchema` (with `PartialEq, Eq` derives) + `stats: bool` flag; conflict gated in dispatcher.
3. ✅ `cmd_profile` `OutputFormat::JsonSchema` arm + `cmd_schema_table` refactor onto helper.
4. ✅ README example migrated.
5. ✅ MCP `profile` json-schema branch (classifies columns once, projects into `TableSchemaColumn`); MCP `schema` table-mode branches return verbatim migration error.
6. ✅ Three golden tests added; iterated assertions against helper's actual contract (validation-derived keywords are independent of `--stats`).
7. ✅ `make ci` clean.

## Notes

- Card 0005's `--schema -` (stdin) ships after this card → ac-10 took the
  documented degraded path: structural-shape assertions on parsed JSON.
- Helper signature is fit-for-purpose for card 0006 type-mode extension
  (separate `emit_type_schema(...)` will live in the same module).
- Three pre-existing `golden_profile_*` failures (titanic SibSp, ecommerce_orders
  order_id, people_directory phone) confirmed pre-existing on clean branch
  state — caused by the v19-relu model promotion (PR #50, merged
  2026-04-27). Out of scope for this card.
