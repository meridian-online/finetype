# Progress — validate absorbs load (card 0005)

Spec: `orbit/specs/2026-04-28-validate-absorbs-load/spec.yaml` (v1.1, 13 ACs)

## Acceptance criteria

- [x] ac-01 — `build_transform_projection` helper (5 unit tests inc. unknown-label branch)
- [x] ac-02 — `cmd_validate_table` materialise path uses projection (DATE/DECIMAL/VARCHAR)
- [x] ac-03 — TRY-wrap transforms; `TRANSFORM_FAILED` reject row (`2024-02-30` case)
- [x] ac-04 — NULL-in-NULL-out NOT a transform failure (empty cell, exit 0, count=3)
- [x] ac-05 — `Commands::Load` / dispatch arm / `cmd_load` deleted; LOC delta -420 (well under -250 target)
- [x] ac-06 — `finetype load` → clap unknown-subcommand error, exit 2 (`test_vrp_load_subcommand_removed`)
- [x] ac-07 — MCP `lib.rs:113` description addendum; `tools/validate.rs` doc-comment-only edit (ValidateRequest unchanged)
- [x] ac-08 — 13 vrp_* CLI tests green (8 prior + 4 new typed-CTAS + 1 unknown-subcommand verification); 7 engine-side untouched
- [x] ac-09 — `golden_load_*` deleted with breadcrumb pointing to `vrp_typed_ctas_round_trip` and projection unit tests
- [x] ac-10 — MADR 0071 created (status=accepted, refines 0064); MADR 0064 date-modified bumped to 2026-04-28; error_message semantic split documented
- [x] ac-11 — README / CLAUDE.md / skills / smoke migration; 5 public commands (infer, profile, validate, mcp, taxonomy)
- [x] ac-12 — CHANGELOG `### Removed` (verbatim migration map) + 3× `### Changed` (typed cols, ENUM drop, ontology gains TRANSFORM_FAILED + transform)
- [x] ac-13 — `make ci` passes (fmt + clippy + test + check; 240/240 taxonomy, 12000/12000 samples)

## Implementation log

1. ac-01 — ported `build_load_expr` logic to `build_transform_projection(headers, extensions, taxonomy, try_wrap)` with 5 unit tests.
2. ac-02 — wired projection into `cmd_validate_table` materialise path.
3. ac-03 + ac-04 — pre-CTAS sweep (`__finetype_transform_failures_<uuid>` temp table) with predicate `col IS NOT NULL AND TRY(transform) IS NULL`; user-table CTAS excludes failed `__row_idx`s via `NOT IN`.
4. ac-05 — deleted `Commands::Load`, dispatch arm, verbose-tracing match, `cmd_load`, `build_load_expr`, `build_load_expr_enum`, orphan `sanitise_identifier`, 6 obsolete unit tests.
5. ac-06 — added `test_vrp_load_subcommand_removed`: asserts exit 2 + clap unknown-subcommand error mentioning "load".
6. ac-08 — 13 CLI vrp_* tests green via `cargo test -p finetype-cli --test validate_cli -- --ignored`.
7. ac-09 — deleted `golden_load_datetime_formats`, `golden_load_ecommerce_orders`, `run_load` helper; left a breadcrumb at the section header.
8. ac-07 — extended MCP `validate` `#[tool(description=...)]` literal at `lib.rs:113` and added module-level doc comment to `tools/validate.rs`. ValidateRequest byte-identical.
9. ac-10 — wrote `decisions/0071-validate-absorbs-load.md` (refines 0064); bumped `decisions/0064-...md` date-modified to 2026-04-28.
10. ac-11 — updated README, CLAUDE.md ("Public vs internal CLI surface" 6→5 verbs), `.claude/skills/finetype-cli/SKILL.md`, `.claude/skills/finetype-pipeline/SKILL.md`, `tests/smoke.sh`.
11. ac-12 — added `### Removed` (load verb retirement, verbatim migration map) and 3× `### Changed` entries (typed cols, ENUM drop, ontology) to CHANGELOG.
12. ac-13 — `make ci` green after a `cargo fmt` pass on the projection unit tests.

## Hand-off — review-pr

Branch: `rally/validate-absorbs-load` (stacked on `rally/schema-verb-fold` → `main`).
8 commits since branch point. Ready for forked review per drive §7.
