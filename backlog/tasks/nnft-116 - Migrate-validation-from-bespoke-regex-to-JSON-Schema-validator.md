---
id: NNFT-116
title: Migrate validation from bespoke regex to JSON Schema validator
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-24 06:52'
updated_date: '2026-02-24 09:19'
labels:
  - architecture
  - validation
  - ecosystem
dependencies: []
documentation:
  - 'https://github.com/sourcemeta/blaze'
  - 'https://github.com/Stranger6667/jsonschema-rs'
  - 'https://json-schema.org/specification'
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The taxonomy YAML validation fields use JSON Schema keywords (pattern, minLength, maxLength, minimum, maximum, enum) but the implementation in finetype-core::validator uses hand-rolled Rust regex matching instead of a proper JSON Schema validator.

**Why this matters:**
- Ecosystem integration: JSON Schema is a standard; proper compliance means FineType validation schemas can be consumed by any JSON Schema-aware tool
- Documentation: Users can reference JSON Schema spec docs rather than our bespoke interpretation
- Future validation: We'll want richer keywords (format, oneOf, if/then, $ref) as accuracy work progresses
- Regex dialect gap: JSON Schema specifies ECMA-262 regex; we use Rust regex crate. Currently no conflicts (patterns use basic constructs), but future patterns could diverge.

**Current state:**
- 169 type definitions with validation schemas in labels/definitions_*.yaml
- ~70 types have regex patterns (checked: all currently compatible between ECMA-262 and Rust regex)
- validator.rs: validate_value(), validate_value_for_label(), validate_column() — all use bespoke logic
- Used by: finetype check command, NNFT-115 attractor demotion (planned)

**Research needed:**
- Evaluate Blaze (https://github.com/sourcemeta/blaze) — high performance JSON Schema validator with published benchmarks. C++ based, may have Rust bindings or FFI options.
- Evaluate jsonschema Rust crate (https://github.com/Stranger6667/jsonschema-rs) — pure Rust, widely used
- Benchmark: validate_value() is called per-value during column classification. Need sub-microsecond per validation for hot path. Current regex approach is fast; JSON Schema overhead needs measurement.
- Compatibility: Ensure all 169 existing validation schemas are valid JSON Schema drafts
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Benchmark current validate_value() performance (calls/sec) as baseline
- [x] #2 Evaluate JSON Schema validator options (Blaze via FFI, jsonschema-rs, others) for correctness and performance
- [x] #3 Replace bespoke validator with JSON Schema-compliant implementation
- [x] #4 All 169 existing validation schemas pass without modification (or document required changes)
- [x] #5 Performance regression < 2x on validate_value() hot path
- [x] #6 validator.rs public API preserved (validate_value, validate_column, etc.)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Phase 1: Add jsonschema dependency (workspace + finetype-core)
Phase 2: Validation::to_json_schema() in taxonomy.rs
Phase 3: CompiledValidator struct in validator.rs
Phase 4: Taxonomy validator cache (compile_validators, get_validator)
Phase 5: Replace validate_value() internals to use jsonschema
Phase 6: Update callers (column.rs attractor demotion, checker.rs, main.rs)
Phase 7: Test all 169 schemas compile
Phase 8: Add new tests for CompiledValidator + preserve existing tests
Phase 9: Build, test, verify no regression
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Phase 1-6 implemented: jsonschema-rs v0.42.1 added, Validation::to_json_schema() conversion, CompiledValidator with pre-compiled JSON Schema + manual numeric bounds, taxonomy validator cache, all callers updated.

Phase 7: All 169 taxonomy schemas compile (test_all_taxonomy_schemas_compile).

Phase 8: 16 new tests added (89 total in finetype-core, up from 73).

Phase 9: All 247 tests pass (89 core + 158 model), clippy clean, fmt clean, profile eval shows no regression.

Note: finetype check now reports 7 validation failures (previously 0) because the checker now validates enum/minimum/maximum constraints that the old hand-rolled checker skipped. These are real pre-existing generator issues, not regressions.

AC#1 (benchmark baseline) and AC#2 (evaluate options) were completed in the research phase — see claude-mem #4989 and #5011.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Migrated FineType's validation engine from hand-rolled regex to JSON Schema-compliant validation using jsonschema-rs v0.42.1.

## What changed

**New: `CompiledValidator` type** (`validator.rs`)
Pre-compiles a JSON Schema once and validates many values without re-compilation. Two methods: `is_valid()` (fast boolean for hot loops) and `validate()` (detailed error reporting). Hybrid strategy: string keywords (pattern, minLength, maxLength, enum) delegated to jsonschema; numeric bounds (minimum, maximum) handled manually to preserve FineType's string→f64 parsing semantics.

**New: `Validation::to_json_schema()`** (`taxonomy.rs`)
Converts a Validation fragment to a proper JSON Schema object. Deliberately excludes minimum/maximum (handled manually).

**New: Taxonomy validator cache** (`taxonomy.rs`)
`compile_validators()` pre-compiles all 169 schemas at startup. `get_validator(label)` returns cached references. Clone drops the cache (jsonschema::Validator doesn't impl Clone).

**Updated: `validate_value()`** (`validator.rs`)
Now delegates to CompiledValidator internally. Public API preserved (AC#6). Column validation compiles once at the top instead of per-value.

**Updated: Attractor demotion** (`column.rs`)
Hot path uses `taxonomy.get_validator()` for pre-compiled validation. Falls back to compile-per-call if cache not populated.

**Updated: Checker** (`checker.rs`)
Pre-compiles validator once per definition. Now validates ALL JSON Schema keywords (including enum, minimum, maximum) — previously only checked pattern, minLength, maxLength. This surfaces 7 pre-existing generator issues that were invisible before.

**Updated: CLI** (`main.rs`)
Calls `taxonomy.compile_validators()` after loading taxonomy in both infer-column and profile commands.

## Files changed
- `Cargo.toml` — added `jsonschema = \"0.42\"` to workspace deps
- `crates/finetype-core/Cargo.toml` — added jsonschema dependency
- `crates/finetype-core/src/taxonomy.rs` — to_json_schema(), validator cache, custom Clone/Debug
- `crates/finetype-core/src/validator.rs` — CompiledValidator, updated internals, 16 new tests
- `crates/finetype-core/src/checker.rs` — uses CompiledValidator, removed regex import
- `crates/finetype-core/src/lib.rs` — exports CompiledValidator, Validation
- `crates/finetype-model/src/column.rs` — attractor demotion uses cached validators
- `crates/finetype-cli/src/main.rs` — compile_validators() at startup
- `CLAUDE.md` — documented decision #12

## Tests
- 89 finetype-core tests (16 new), 158 finetype-model tests — all pass
- clippy clean, fmt clean
- Profile eval: no regression (68/74 format-detectable correct)
- All 169 taxonomy schemas compile successfully"
<!-- SECTION:FINAL_SUMMARY:END -->
