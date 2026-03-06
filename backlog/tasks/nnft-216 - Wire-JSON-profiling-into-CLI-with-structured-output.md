---
id: NNFT-216
title: Wire JSON profiling into CLI with structured output
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-04 20:15'
updated_date: '2026-03-06 03:23'
labels:
  - cli
  - json
milestone: m-9
dependencies:
  - NNFT-209
references:
  - crates/finetype-cli/src/main.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extend `finetype profile` command to handle JSON/NDJSON input with structured output. Auto-detects file format by extension, reads via json_reader module, classifies each path using ColumnClassifier (with path leaf as header hint), and produces output that preserves JSON structure for JSON format while flattening for plain/CSV.

Key output distinction:
- JSON output: reconstructs nested structure showing full path hierarchy
- Plain/CSV output: flat list of paths with classification results (natural for terminals/pipelines)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 JSON/NDJSON auto-detection by .json/.ndjson/.jsonl extension
- [x] #2 json_reader module used for both single documents and NDJSON line-by-line reading
- [x] #3 Path leaf used as header hint for ColumnClassifier (e.g., 'email' for 'users[].email')
- [x] #4 JSON output reconstructs nested structure showing full path hierarchy with type/confidence at each level
- [x] #5 Plain output shows flat list of paths (natural for terminal/pipeline consumption)
- [x] #6 CSV output includes path, type, broad_type, confidence columns
- [x] #7 All output formats work correctly with JSON input files
- [x] #8 Clear error message on malformed JSON input
- [x] #9 Depends on NNFT-209 (json_reader module)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Strategy

### Phase 1: Extension Detection & JSON Reading
1. Modify finetype profile command handler to detect JSON/NDJSON by file extension
2. Add file extension matching: .json, .ndjson, .jsonl → use json_reader
3. Integrate json_reader::collect_json() and collect_ndjson() calls
4. Error handling: malformed JSON → clear error message

### Phase 2: Path Classification
1. For each path in JsonPathMap, extract values
2. Extract path leaf as header hint: "users[].email" → header = "email"
3. Create ColumnClassifier for each path (using values and header hint)
4. Store results: {path → (label, broad_type, confidence)}

### Phase 3: Output Formatting
1. **Plain/CSV output (flat):**
   - Iterate paths in order
   - Output: path | type | broad_type | confidence
   - Same structure as CSV profile output
2. **JSON output (structured):**
   - Reconstruct JSON tree from flat paths
   - Inject type/confidence at each path location
   - Example: {"users": [{"email": {"type": "...", "confidence": 0.95}}]}

### Phase 4: Testing & Verification
1. Create test JSON files: simple object, nested, arrays, schema evolution
2. Test all output formats with each input type
3. Verify classification results match expected types
4. Smoke tests: edge cases (empty arrays, deeply nested, malformed JSON)

### Key Design Decisions
- Path leaf extraction: split on . and [], take rightmost non-empty part
- For arrays (users[]), use "users" as header hint
- For nested array items (users[].address.city), use "city" as header hint
- JSON reconstruction uses recursive descent from flat paths
- Classification reuses existing ColumnClassifier (no changes needed there)
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
All ACs verified with fresh evidence:
- JSON/NDJSON auto-detection: .json and .ndjson files correctly routed
- Path leaf header hints: email→identity.person.email, city→geography.location.city
- JSON output: nested address.city/country reconstructed into address object
- Plain/CSV output: flat paths (address.city, address.country)
- Malformed JSON: clear error message
- CSV regression: 14/14 columns typed on existing dataset
- 258/258 tests passing, taxonomy check clean
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Extended `finetype profile` to handle JSON and NDJSON input with structured output.

## What Changed

**New input support:**
- Auto-detects .json/.ndjson/.jsonl files by extension
- JSON arrays treated as multi-row (each element = one document)
- NDJSON read line-by-line with schema evolution
- Uses json_reader module (NNFT-209) for path collection

**Path classification:**
- Extracts path leaf as header hint: `users[].address.city` → `city`
- Existing ColumnClassifier pipeline (Sense→Sharpen) handles classification

**Structured output (JSON input + -o json):**
- Adds `schema` field that reconstructs nested JSON hierarchy
- Example: flat path `address.city` becomes `{"address": {"city": {"type": "...", "confidence": "..."}}}`
- Array paths get `_array: true` and `_items` container
- Flat `columns` array also included for programmatic access

**Flat output (plain/CSV):**
- Paths displayed as-is: `address.city`, `users[].email`
- Same format as CSV profile output

**Error handling:**
- Clear error for malformed JSON: `Malformed JSON in "file.json": <serde error>`
- Scalar JSON rejection: `JSON input must be an object or array of objects`

**Refactored:**
- Extracted `read_json_input()`, `read_csv_input()`, `path_leaf()`, `reconstruct_json_schema()`, `insert_path()` helper functions
- CSV profiling unchanged (no regression)

## Tests
- cargo test --lib: 258/258 passing
- cargo run -- check: 216/216 definitions passing
- Manual verification: JSON, NDJSON, CSV inputs × plain, json, csv outputs
- Malformed JSON and scalar JSON error cases verified
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
