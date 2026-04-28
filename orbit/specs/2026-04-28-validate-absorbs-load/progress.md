# Progress — validate absorbs load (card 0005)

Spec: `orbit/specs/2026-04-28-validate-absorbs-load/spec.yaml` (v1.1, 13 ACs)

## Acceptance criteria

- [ ] ac-01 — `build_transform_projection` helper (5 unit tests inc. unknown-label branch)
- [ ] ac-02 — `cmd_validate_table` materialise path uses projection (DATE/DECIMAL/VARCHAR)
- [ ] ac-03 — TRY-wrap transforms; `TRANSFORM_FAILED` reject row (`2024-02-30` case)
- [ ] ac-04 — NULL-in-NULL-out NOT a transform failure (empty cell, exit 0, count=3)
- [ ] ac-05 — `Commands::Load` / dispatch arm / `cmd_load` deleted; LOC delta ≤ -250
- [ ] ac-06 — `finetype load` → clap unknown-subcommand error, exit 2
- [ ] ac-07 — MCP `lib.rs:113` description addendum; `tools/validate.rs` only doc-comment edit
- [ ] ac-08 — 15 vrp_* tests green (8 CLI + 7 engine — count holds; engine-side untouched)
- [ ] ac-09 — `golden_load_*` deleted or replaced
- [ ] ac-10 — MADR 0071 created; MADR 0064 date-modified bumped; error_message semantic split documented
- [ ] ac-11 — README / CLAUDE.md / skills migration; 5 public commands
- [ ] ac-12 — CHANGELOG `### Removed` (migration map) + `### Changed` (typed cols, ENUM drop, ontology)
- [ ] ac-13 — `make ci` passes (PR #54 baseline)

## Implementation order

1. ac-01 — port `build_load_expr` logic to `build_transform_projection` with 5 unit tests
2. ac-02 — wire projection into `cmd_validate_table` materialise path
3. ac-03 + ac-04 — transform-failure pre-CTAS sweep + reject-row INSERT (binding choice)
4. ac-05 — delete `Commands::Load`, dispatch arm, `cmd_load`, `build_load_expr`, `build_load_expr_enum`
5. ac-06 — verify clap unknown-subcommand path (no code; pure test)
6. ac-08 — run `validate_cli.rs` to confirm 8+4=12 tests green
7. ac-09 — delete or replace `golden_load_*`
8. ac-07 — MCP `lib.rs:113` description string addendum + `tools/validate.rs` doc comment
9. ac-10 — MADR 0071 + MADR 0064 date-modified bump
10. ac-11 — README, CLAUDE.md, .claude/skills migration
11. ac-12 — CHANGELOG entries
12. ac-13 — `make ci` gate
