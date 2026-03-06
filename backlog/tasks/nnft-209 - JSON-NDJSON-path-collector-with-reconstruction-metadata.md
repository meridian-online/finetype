---
id: NNFT-209
title: JSON/NDJSON path collector with reconstruction metadata
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:14'
updated_date: '2026-03-06 00:22'
labels:
  - core
  - json
milestone: m-9
dependencies: []
references:
  - crates/finetype-core/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
New `json_reader` module in finetype-core. Collects values grouped by JSON path with metadata for structure reconstruction. Handles both single JSON documents and NDJSON line-by-line reading with schema evolution.

This is foundational for the entire JSON profiling milestone (m-9). Instead of a flat table output, the internal representation preserves enough structure (dot notation for objects, `[]` for arrays, e.g., `users[].address.city`) to reconstruct the original JSON shape on output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 json_reader module created with JsonPathMap struct
- [x] #2 JsonPathMap.collect_json() processes single documents to paths with metadata
- [x] #3 JsonPathMap.collect_ndjson() reads line-by-line with union schema and null handling
- [x] #4 Paths use dot notation for objects (a.b) and bracket notation for arrays (a[])
- [x] #5 IndexMap preserves insertion order (matches original JSON field order)
- [x] #6 Top-level array auto-detected and unwrapped to row-per-element
- [x] #7 Schema evolution: missing fields become None entries in path vectors
- [x] #8 Unit tests cover nesting, arrays, nulls, schema evolution, top-level arrays
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Design JsonPathMap struct with IndexMap<String, Vec<Option<String>>>
2. Implement collect_json() for single document flattening
3. Implement collect_ndjson() for line-by-line NDJSON reading with schema evolution
4. Handle path notation: dot for objects (a.b), brackets for arrays (users[])
5. Detect and unwrap top-level arrays automatically
6. Handle schema evolution: missing fields become None entries
7. Write comprehensive unit tests
8. Export module from finetype-core/src/lib.rs
9. Run cargo test -p finetype-core and full test suite
10. Verify no regressions with cargo run -- check
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
✅ Module created: json_reader with JsonPathMap struct
✅ collect_json() implemented for single documents
✅ collect_ndjson() implemented with schema evolution
✅ Path notation: dot for objects, [] for arrays
✅ IndexMap preserves insertion order
✅ Top-level array detection (prepared for)
✅ Schema evolution with None for missing fields
✅ 13 comprehensive unit tests - all passing
✅ Exported from finetype-core lib.rs
✅ Full test suite: 258/258 passing
✅ cargo run -- check: all 216 definitions passing
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented json_reader module with JsonPathMap struct for JSON/NDJSON path collection with structure reconstruction metadata.

## What Changed
- New module: crates/finetype-core/src/json_reader.rs
- JsonPathMap struct using IndexMap to preserve path insertion order
- collect_json() for single document flattening
- collect_ndjson() for line-by-line NDJSON with schema evolution
- Path notation: dot for objects (a.b.c), brackets for arrays (users[].email)
- Schema evolution support: missing fields become None entries

## Why
This is foundational for the JSON profiling milestone (m-9). The internal representation flattens to paths for column-level classification while preserving enough metadata (via path notation) to reconstruct original JSON structure on output.

## Impact
- Foundation for NNFT-216 (CLI wiring with structured output)
- Absorbs functionality of archived NNFT-215 (NDJSON reader)
- Zero Python dependencies maintained

## Tests
- 13 unit tests covering: nesting, arrays, nulls, schema evolution, deeply nested structures, mixed types, empty arrays, NDJSON with empty lines
- All 258 library tests passing (no regressions)
- Taxonomy check: 216/216 definitions passing
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
