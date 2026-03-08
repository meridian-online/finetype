---
id: NNFT-258
title: >-
  Expand golden tests — structured CLI regression suite for profile, load,
  taxonomy, schema
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-08 06:45'
updated_date: '2026-03-08 09:38'
labels:
  - testing
  - quality
dependencies:
  - NNFT-254
references:
  - tests/golden/
  - tests/smoke.sh
  - tests/helpers.sh
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Why:** FineType has only 3 golden tests (all locale-specific single-value inference). The profile and load commands — the two most analyst-facing features — have zero regression coverage. A single obvious misclassification (e.g., unix_epoch→npi, shipping_postal_code→cpt) destroys trust in the entire tool. We need a structured test suite that locks in correct behavior across all user-facing CLI commands.

**Context from interview (2026-03-08):**
- Golden tests should cover 4 CLI commands: `profile`, `load`, `taxonomy`, `schema`
- Match mode: structured field matching (type label, broad_type, confidence band) — not byte-for-byte snapshot comparison
- Test data: both small focused fixtures (5-10 rows, edge cases) AND real-world CSVs from ~/datasets/
- Test runner: Rust integration tests (`cargo test`), gated behind `#[ignore]` or feature flag for heavyweight model-loading tests
- Sequenced after NNFT-254 accuracy spike so we lock in improved behavior

**Datasets to cover (at minimum):**
- datetime_formats.csv — diverse temporal types, known epoch misclassification
- ecommerce_orders.csv — mixed domains, known postal_code→cpt issue
- titanic.csv — classic dataset, known Cabin→icd10 issue
- people_directory.csv — identity types, age/salary edge cases
- Small focused fixtures for: ambiguous headers, mixed-type columns, single-char categoricals
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Rust integration test suite exists under tests/ covering profile, load, taxonomy, and schema commands
- [x] #2 Profile tests use structured field matching — assert type label and broad_type per column, not byte-for-byte output comparison
- [x] #3 At least 4 real-world CSV datasets have profile golden tests (datetime_formats, ecommerce_orders, titanic, people_directory)
- [x] #4 At least 3 small focused fixture CSVs test edge cases (ambiguous headers, numeric-only columns, single-char categoricals)
- [x] #5 Load tests verify the generated DuckDB DDL contains correct column types (DATE, TIMESTAMP, BIGINT etc. — not VARCHAR for typed columns)
- [x] #6 Taxonomy and schema command tests verify output structure and key fields
- [x] #7 Tests gated appropriately (feature flag or #[ignore]) so cargo test stays fast for dev workflow
- [x] #8 All tests pass in CI
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### 1. Test infrastructure
- Create `tests/cli_golden.rs` as a Rust integration test file in the `finetype-cli` crate
- Add `assert_cmd` + `predicates` dev-dependencies for CLI testing
- Gate all tests with `#[ignore]` (they load the model — ~3s per test)
- Helper functions: `run_profile_json(csv_path)`, `run_load(csv_path)`, `run_taxonomy_json()`, `run_schema(type_key)`

### 2. Profile golden tests — real-world CSVs (AC #2, #3)
Parse `profile -f <file> -o json` output and assert structured fields per column:
- `datetime_formats.csv` — 14 columns, all datetime types
- `ecommerce_orders.csv` — 12 columns, mixed domains
- `titanic.csv` — 12 columns, known edge cases (Age, Cabin)
- `people_directory.csv` — 14 columns, identity types

For each column: assert `type` label and `broad_type`. Don't assert exact confidence (model-dependent).

### 3. Profile golden tests — focused fixtures (AC #4)
Create 3 small fixture CSVs under `tests/fixtures/`:
- `ambiguous_headers.csv` — columns with names like \"id\", \"code\", \"value\", \"status\" (5 rows)
- `numeric_edge_cases.csv` — integers, decimals, amounts, zip-like codes (5 rows)
- `categoricals.csv` — boolean Y/N, single-char codes, low-cardinality text (5 rows)

### 4. Load golden tests (AC #5)
Run `load -f <file>` and check the DDL output:
- datetime_formats: DATE, TIMESTAMP, BIGINT, TIME, VARCHAR types
- No VARCHAR for typed columns (except truly generic ones)

### 5. Taxonomy + schema tests (AC #6)
- `taxonomy --output json`: verify it returns an array with 250 entries, each having `key`, `broad_type`, `title`
- `schema identity.person.email --pretty`: verify JSON Schema structure (`$schema`, `pattern`, `type`, `x-finetype-broad-type`)

### 6. CI gating (AC #7)
- All tests marked `#[ignore]`
- Can run with `cargo test -- --ignored` or `cargo test --test cli_golden`
- Add a note in Makefile/CI about running golden tests
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added structured golden integration test suite for the FineType CLI, covering profile, load, taxonomy, and schema commands.

## What changed

**New file: `crates/finetype-cli/tests/cli_golden.rs`** — 13 integration tests:
- 4 profile tests on real-world datasets (datetime_formats, ecommerce_orders, titanic, people_directory)
- 3 profile tests on focused fixture CSVs (ambiguous_headers, numeric_edge_cases, categoricals)
- 2 load tests verifying DuckDB DDL output (correct types, transforms, all_varchar)
- 2 taxonomy tests (250 types, domain counts, required fields)
- 2 schema tests (JSON Schema structure, FineType extensions, PII flag)

**3 fixture CSVs:** `tests/fixtures/ambiguous_headers.csv`, `numeric_edge_cases.csv`, `categoricals.csv`

**Test approach:** Structured field matching via JSON parsing — asserts type labels, broad_types, and domain prefixes per column. No byte-for-byte snapshot comparison. Uses `std::process::Command` (no new crate dependencies beyond serde_json dev-dep).

**Gating:** All tests marked `#[ignore]` — `cargo test` stays fast (~6s). Run golden tests explicitly with `cargo test -p finetype-cli --test cli_golden -- --ignored` (~113s, loads model for each test).

## Tests

- 13/13 golden tests pass
- 438 existing tests unaffected
- Regular `cargo test` shows 13 ignored (not run)
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
