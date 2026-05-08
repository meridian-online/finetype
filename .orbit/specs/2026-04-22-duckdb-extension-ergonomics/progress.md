# Implementation Progress

Spec path: .orbit/specs/2026-04-22-duckdb-extension-ergonomics/spec.yaml
Spec hash: sha256:33fefc1fd2be227b42ac482141362fef995d74897f85cae25621d5efffd8ba2f
Started: 2026-04-24
Current AC: ac-14

## Hard Constraints
- [x] Validation is deterministic. Given a JSON Schema and a row, pass/fail is a pure function. No heuristics that second-guess the schema at validation time.
- [x] Schema lives in JSON files — reviewed, edited, checked into git. No catalog state inside DuckDB. CLI authors schemas via `finetype schema`; DuckDB extension only consumes them.
- [x] CLI artefact is a DuckDB `.db` file. No intermediate Parquet or CSV export. Analyst opens the database and queries tables in-situ.
- [x] Reject columns mirror DuckDB `reject_errors` base (scan_id, file_id, line, column_idx, column_name, error_type, csv_line, byte_position, error_message) with FineType extensions (type_confidence, expected_type, constraint_failed, constraint_value). UNION across native + FineType rejects requires explicit NULL-pad projection; that cost is accepted.
- [x] Single validation engine: finetype-core `table_validator::validate_table` is the sole source of pass/fail + reject detail. The CLI is its only caller in the default flow; no new DuckDB functions are registered. The existing scalar `finetype_validate(value, schema_json)` remains untouched for ad-hoc SQL use.
- [x] CLI owns sidecar materialisation. `finetype_reject_errors` is created (or appended to) by the CLI as a persistent table in the output `.db`. The validation engine returns a structured result; the CLI projects it into SQL rows.
- [x] scan_id is a per-invocation counter assigned by the CLI. Fresh `.db` → scan_id starts at 1. `--append` into an existing `.db` with a pre-existing sidecar → scan_id = MAX(existing.scan_id) + 1. Concurrent writers are NOT supported in v1; the CLI documents sequential use.
- [x] The `finetype schema <file>` JSON Schema output format is an input contract for this spec. The spec depends on `x-finetype-confidence`, `x-finetype-label`, and the pattern/length/enum/required constraints being present in schemas emitted by that command. Any change to the schema-emission format is a breaking change that must update this spec.
- [x] The additive change to `finetype_core::table_validator::TableValidationResult` is intra-workspace only (consumers: finetype-cli, finetype-mcp, finetype-duckdb). All three crates are updated in the same PR. No external Rust consumer surface exists for this type today; `finetype-core` is not published under a SemVer contract for this struct.

## Detours
2026-04-24: ac-04 spike ratified Scenario A; spec rewritten to v1.2 (DuckDB table function dropped, CLI calls table_validator directly; see spike-duckdb-rs.md). Drift acknowledged — hash updated to v1.2.
Return to: ac-01

## Acceptance Criteria
- [x] ac-01 (gate): TableValidationResult gains valid_row_indices + rejects fields (additive); RejectRecord struct added — test_vrp_ac01_result_shape passes; workspace builds
- [x] ac-02: Each jsonschema validator failure maps to RejectRecord with canonical constraint_failed token — 6 tests pass (pattern/min_length/max_length/enum/type/required)
- [x] ac-03: validate_table is deterministic — byte-identical output on repeat calls; rejects sorted by (row_index, column_index) — test_vrp_ac03_determinism + validator/missing_columns sorted explicitly
- [x] ac-04 (gate): duckdb-rs vtab spike landed — spike.rs registers finetype_spike; spike-duckdb-rs.md documents findings (a) vtab available, (b) same-name compile-proven, (c) table-name VTab BLOCKED in safe API → Scenario A ratified
- [x] ac-05: Scenario A pivot documented — spike.rs docstring references MADR 0064; lib.rs entrypoint comment names finetype_spike as spike artefact (not production)
- [x] ac-06: CLI projects TableValidationResult into .db via two INSERTs (valid rows + finetype_reject_errors) — covered by test_vrp_ac13_cli_writes_db_with_sidecar: 13-col ontology verified via duckdb_columns, SEMANTIC_TYPE marker present, user table populated with valid-only rows
- [x] ac-07: Existing scalar finetype_validate(value, schema_json) preserved unchanged; test_vrp_ac07_scalar_unchanged + test_vrp_ac07_schema_error_prefixed pass
- [x] ac-08: Malformed JSON Schema surfaces as CLI error exit 2 before any .db write; four integration tests — test_vrp_ac13_cli_malformed_schema_error_grid covers missing-file / invalid-JSON / missing-properties / permission-denied; each asserts !db.exists() after exit 2
- [x] ac-09 (gate): CLI flow `finetype validate <input> <schema.json> --db <output.db> --table <name>` with 10-step execution + RAII staging cleanup on success AND failure — cmd_validate_table shells out to duckdb CLI with a TEMPORARY staging table (auto-drop on session exit = RAII cleanup). test_vrp_ac13_cli_staging_cleanup_on_success + _on_failure assert no staging residue and no partial .db
- [x] ac-10: Exit codes 0/1/2 with --lenient override (doesn't affect error exit 2) — test_vrp_ac13_cli_exit_code_grid covers all four cells: zero-reject=0, reject-no-lenient=1, reject-with-lenient=0, error-with-lenient=2
- [x] ac-11: Schemas from `finetype schema` work unmodified; x-finetype-confidence/x-finetype-label surface as type_confidence/expected_type with graceful NULL on absence — test_vrp_ac11_xft_extensions_surface (populated case: 'identity.code.id|0.99') + test_vrp_ac11_null_on_absence (all-NULL case)
- [x] ac-12: End-to-end ecommerce_orders test — authored-time confidence distinguishes "classifier wrong" from "data bad" — test_vrp_ac12_ecommerce_end_to_end asserts ≥1 high-confidence reject (type_confidence ≥ 0.99) on the fixture slice and scan_id=1 on fresh db
- [x] ac-13 (gate): Test grid — 15 named vrp_acNN_* functions across 2 crates (core 7 + CLI 8), all passing. finetype-core: test_vrp_ac01_result_shape, test_vrp_ac02_{pattern,min_length,max_length,enum,type,required}_failure, test_vrp_ac03_determinism, test_vrp_ac13_{happy_all_valid,all_reject,partial_reject_mixed,multi_reject_per_row,empty_input,single_row_single_column}. finetype-cli: test_vrp_ac11_{xft_extensions_surface,null_on_absence}, test_vrp_ac12_ecommerce_end_to_end, test_vrp_ac13_{cli_writes_db_with_sidecar,cli_staging_cleanup_on_success,cli_staging_cleanup_on_failure,cli_exit_code_grid,cli_malformed_schema_error_grid}
- [x] ac-14: Docs updated — CLAUDE.md CLI table row rewritten (line 180), README CLI section gained a 4-line validate+duckdb example, cards 0005 & 0007 already reference this spec in their specs arrays, MADR 0064 created (refines 0031, 0032)

## Notes

Hugh authorised spike-first execution (2026-04-24). Working order diverges
from spec-declaration order at the gate-enforcement layer: ac-04 ran before
ac-01/02/03. Spike outcome ratified rollback_plan Scenario A (see
spike-duckdb-rs.md + spec v1.2 changelog). Resuming ac-01/02/03 now.

Drift acknowledgement (2026-04-24): spec v1.1 → v1.2 rewrite was explicitly
anticipated by v1.1's rollback_plan (Scenario A). Hash updated; the v1.2
shape is the implementation target.
