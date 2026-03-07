---
id: NNFT-242
title: Remove low-precision integer-range types — http_status_code and port
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 03:53'
updated_date: '2026-03-07 04:21'
labels:
  - taxonomy
  - precision
dependencies: []
references:
  - labels/definitions_technology.yaml
  - crates/finetype-model/src/column.rs
  - crates/finetype-model/src/label_category_map.rs
  - crates/finetype-core/src/generators/
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Remove `technology.internet.http_status_code` and `technology.internet.port` from the taxonomy. Both cause false positives where plain integer columns get misclassified.

These are small integer ranges (status: 100-599, port: 0-65535) that overlap heavily with `integer_number`. Per the Precision Principle: "A validation that confirms 90% of random input is not a validation."

Before finalising, review eval data to check whether other integer-range types have the same false-positive problem.

Taxonomy-only change — no model retrain. Retrain deferred to a later batch with other planned taxonomy changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 technology.internet.http_status_code removed from definitions_technology.yaml
- [x] #2 technology.internet.port removed from definitions_technology.yaml
- [x] #3 Corresponding generators removed or updated
- [x] #4 References in column.rs (header hints, disambiguation rules) cleaned up if present
- [x] #5 label_category_map.rs updated if these types are referenced
- [x] #6 `cargo run -- check` passes with updated type count (209 → 207)
- [x] #7 `cargo test` and `make ci` pass
- [x] #8 Eval data reviewed for other integer-range false-positive offenders — findings documented in task notes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## NNFT-242 Implementation Plan

### 1. Remove type definitions from taxonomy YAML
- Delete `technology.internet.http_status_code` block from `labels/definitions_technology.yaml`
- Delete `technology.internet.port` block from `labels/definitions_technology.yaml`

### 2. Remove generators
- Remove `("internet", "http_status_code")` arm in `crates/finetype-core/src/generator.rs`
- Remove `("internet", "port")` arm in `crates/finetype-core/src/generator.rs`

### 3. Clean up column.rs
- Remove `"port"` header hint (line ~2129-2131)
- Remove `"technology.internet.port"` from `disambiguate_numeric` numeric_types array (line ~2480)
- Remove entire port detection logic block (common_ports, has_common_ports, the port return branch ~2525-2606)
- Remove/update 5 port-related tests: test_port_detection, test_year_not_triggered_for_ports, test_age_column_not_detected_as_port, test_age_column_with_mixed_values_not_port, test_is_generic_numeric_postal_code_detection (port assertion)
- No http_status_code references in column.rs to clean

### 4. Clean up label_category_map.rs
- Remove `"technology.internet.http_status_code"` and `"technology.internet.port"` entries

### 5. Clean up DuckDB extension
- Remove entries in `crates/finetype-duckdb/src/type_mapping.rs` (lines 85-86)
- Remove entries in `crates/finetype-duckdb/src/normalize.rs` (lines 398-406)

### 6. Clean up training data
- Remove entries in `crates/finetype-train/src/data.rs` (lines 371-372)
- Remove Model2Vec prep entries in `crates/finetype-train/src/model2vec_prep.rs` (port, status code, response code, http status)

### 7. Update eval mappings
- Remove http_status_code and port entries from `eval/schema_mapping.csv` and `eval/schema_mapping.yaml`

### 8. Verify
- `cargo run -- check` (expect 207 types)
- `cargo test`
- `cargo clippy`

### 9. Review eval data for other integer-range false-positive offenders
- Check which other types have integer-like overlap (e.g., year, postal_code, increment already have heuristics — are they working?)
- Document findings in task notes
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Integer-range false-positive review (AC #8):**

Remaining integer-typed types in taxonomy:
- `datetime.component.year` (SMALLINT, broad_numbers) — robust disambiguator: ≥80% values must be 4-digit in 1900-2100 range. Working well.
- `representation.numeric.integer_number` (BIGINT, universal) — catch-all, by definition cannot false-positive.
- `representation.file.file_size` (BIGINT, universal) — requires unit suffixes (MB, GB), structurally distinct.
- `representation.identifier.increment` (BIGINT, broad_numbers) — sequential detection heuristic (low variance in diffs). Working well.

**Conclusion:** No other integer-range types have the port/http_status_code problem. The remaining types either have strong heuristics or are structurally distinct from plain integers.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Removed `technology.internet.http_status_code` and `technology.internet.port` from the FineType taxonomy to eliminate false positives on plain integer columns.

**Changes:**
- Removed 2 type definitions from `labels/definitions_technology.yaml`
- Removed generators from `crates/finetype-core/src/generator.rs`
- Removed port header hint, port detection logic, and 5 port-related tests from `crates/finetype-model/src/column.rs`
- Removed entries from `label_category_map.rs` (updated counts: 207 total, 24 numeric, 27 numeric eligible)
- Removed DuckDB type mapping and normalization entries
- Removed training data category mappings and Model2Vec prep entries
- Removed eval schema mapping entries

**Integer-range review:** Confirmed no other integer-range types have the same false-positive problem. Year, increment, postal_code all have robust heuristics. File_size requires unit suffixes.

**Tests:** `cargo run -- check` (207/207, 100%), `cargo test` (405 passed), `cargo clippy` clean."
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
