---
id: NNFT-109
title: Unify finetype() with column-level disambiguation via chunk-aware scalar path
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 11:04'
updated_date: '2026-02-18 12:50'
labels:
  - accuracy
  - duckdb
  - feature
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement a DuckDB scalar function that classifies an entire column by accepting a LIST<VARCHAR> of values, running CharCNN per-value votes, and applying the existing ColumnClassifier disambiguation rules.

Usage pattern:
```sql
SELECT col_name, finetype_column(list(col_value)) FROM t GROUP BY col_name;
SELECT col_name, finetype_column(list(col_value), col_name) FROM t GROUP BY col_name;  -- with header hint
```

This is the highest-ROI accuracy improvement available — per-value classification has a ~55% domain accuracy ceiling because individual values are ambiguous (e.g., "42" could be age, year, zip, ID). Column-level disambiguation rules break through that ceiling.

Design decision: Implemented as scalar-over-LIST instead of a raw C API aggregate. The DuckDB Rust crate v1.4.4 has no high-level aggregate trait, and the raw C API approach would require complex unsafe state management. The scalar-over-LIST approach gives identical results with much simpler code: `finetype_column(list(val))` vs `finetype_column(val)` — only 6 extra characters for the user.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 finetype_column(list(values)) scalar function registered in DuckDB extension, accepting LIST<VARCHAR> input
- [x] #2 finetype_column(list(values), header) overload registered for header-hint disambiguation
- [x] #3 finetype_column_detail(list(values)) registered, returning JSON with type, confidence, votes, disambiguation info
- [x] #4 read_list_varchar() helper correctly extracts string values from DuckDB LIST vectors including NULL handling
- [x] #5 GlobalClassifierDelegate bridges the global CharClassifier to the ColumnClassifier via ValueClassifier trait
- [x] #6 All existing disambiguation rules (13 rules: date formats, coordinates, boolean, categorical, numeric, etc.) active via ColumnClassifier
- [x] #7 Eval SQL updated to use finetype_column() alongside existing finetype() results
- [x] #8 Benchmark results compared: finetype_column() vs finetype() majority vote on both GitTables and SOTAB
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Explore DuckDB Rust crate for aggregate function support — found no high-level aggregate trait
2. Design alternative: scalar function accepting LIST<VARCHAR> via DuckDB's list() aggregate
3. Create column_fn.rs module with:
   - GlobalClassifierDelegate implementing ValueClassifier (bridges to global CharClassifier)
   - read_list_varchar() helper for LIST<VARCHAR> extraction from DuckDB vectors
   - FineTypeColumn VScalar with 2 overloads (values only, values + header)
   - FineTypeColumnDetail VScalar with 2 overloads (same, returns JSON)
4. Register new functions in extension entrypoint
5. Build, test disambiguation (dates, coordinates, boolean, categorical)
6. Update eval SQL to use finetype_column() for column-level benchmarks
7. Run benchmarks on GitTables and SOTAB
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation complete for core function. Key decisions:

- Scalar-over-LIST instead of raw C API aggregate. The duckdb Rust crate v1.4.4 has no aggregate trait; raw C API would require 5 unsafe callbacks (state_size, init, update, combine, finalize) with manual heap state management. The scalar approach reuses existing VScalar infrastructure.

- GlobalClassifierDelegate pattern: thin struct implementing ValueClassifier that delegates to the global OnceLock<CharClassifier>. This bridges the 'static classifier reference into a Box<dyn ValueClassifier> for ColumnClassifier::with_defaults().

- read_list_varchar() reads DuckDB LIST<VARCHAR> vectors using raw C API: reads duckdb_list_entry (offset+length) from parent vector, then reads duckdb_string_t from child vector. Handles NULL at both list and element level.

- All 13 existing disambiguation rules from column.rs are active: date format (us_slash/eu_slash, short_mdy/short_dmy), coordinate (lat/lon range), IPv4, day-of-week, month names, boolean subtypes (binary/terms/initials), boolean override, small-integer ordinal, categorical, numeric range, SI number.

- Two function families: finetype_column (returns label VARCHAR) and finetype_column_detail (returns JSON with type, confidence, duckdb_type, samples, disambiguation, votes).

- Each family has 2 overloads: (LIST<VARCHAR>) and (LIST<VARCHAR>, VARCHAR header).

Tests verified: ISO dates, emails with header hint, GROUP BY pattern, zip codes with header hint, NULL/empty handling, boolean disambiguation, US slash date disambiguation, latitude/longitude coordinate disambiguation."

Design evolved during implementation based on user feedback:

1. Initial design: separate finetype_column(list()) and finetype_column_detail(list()) functions
2. User concern: two function names (finetype vs finetype_column) confuse users into thinking they do different things
3. User insight: the real difference is sampling ratio (1 value vs N values), not a different operation
4. Final design: unified finetype() that automatically uses the DuckDB chunk (~2048 rows) as a column sample for disambiguation in the scalar path. The list() overload remains for explicit control (GROUP BY, header hints).

Technical changes for unification:
- column_fn.rs refactored from standalone VScalar structs to exported helpers (is_list_input, classify_column, invoke_column_label, invoke_column_detail, format_column_result_json)
- FineType::invoke() detects input type via duckdb_vector_get_column_type() → dispatches to column path for LIST<VARCHAR>, chunk-aware column classification for VARCHAR
- FineTypeDetail::invoke() same pattern
- Explicit NULL handling added (overloaded signatures don't get automatic NULL propagation)
- finetype_column and finetype_column_detail registrations removed from entrypoint

Benchmark results (from prior session, before unification — same underlying ColumnClassifier):
- SOTAB: Label 22.2%→24.1%, Domain 42.8%→44.0%, Fixed 346, Broken 30
- GitTables: Label 2.4%→3.1%, Domain 51.0%→52.8%, Fixed 182, Broken 14
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Unified `finetype()` with automatic column-level disambiguation.

Previously `finetype(col)` classified each value independently (sample size = 1). Now the scalar path automatically uses the DuckDB processing chunk (~2048 rows) as a sample for column-level disambiguation — majority vote + 13 disambiguation rules (date formats, coordinates, boolean subtypes, categorical detection, numeric range, etc.) are applied without the user needing to change their query.

The `list()` overload (`finetype(list(col))`, `finetype(list(col), header)`) remains as a power-user escape hatch for explicit control over the sample — useful with GROUP BY or when providing header hints. Same overloads exist for `finetype_detail()`.

Removed the separate `finetype_column` and `finetype_column_detail` functions entirely. The DuckDB extension now exposes 5 functions: `finetype_version`, `finetype`, `finetype_detail`, `finetype_cast`, `finetype_unpack`.

Key implementation details:
- Type detection via `duckdb_vector_get_column_type()` + `duckdb_get_type_id()` to dispatch VARCHAR vs LIST<VARCHAR> in a single `invoke()`
- column_fn.rs exports helpers (`is_list_input`, `classify_column`, `invoke_column_label`, `invoke_column_detail`) rather than standalone VScalar structs
- Explicit `set_null()` calls for NULL inputs — overloaded signatures don't get automatic NULL propagation

Changes:
- `crates/finetype-duckdb/src/column_fn.rs` — new module with column classification helpers
- `crates/finetype-duckdb/src/lib.rs` — unified FineType/FineTypeDetail with type detection dispatch
- `eval/gittables/eval_1m.sql` — section 10 uses `finetype(list())`
- `eval/sotab/eval_sotab.sql` — section 8 uses `finetype(list())`

Commit: 7cb4cd0
<!-- SECTION:FINAL_SUMMARY:END -->
