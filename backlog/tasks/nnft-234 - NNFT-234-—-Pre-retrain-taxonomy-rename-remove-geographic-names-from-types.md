---
id: NNFT-234
title: 'NNFT-234 — Pre-retrain taxonomy rename: remove geographic names from types'
status: Done
assignee:
  - nightingale
created_date: '2026-03-06 09:10'
updated_date: '2026-03-06 09:37'
labels:
  - refactor
  - taxonomy
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rename 10 types that encode geography (eu_*, us_*, american, european) to describe format structure instead. Follows naming convention: "describe the format, not the locale." 

Target renames:
- Dates: eu_slash→dmy_slash, eu_dot→dmy_dot, eu_short_slash→dmy_short_slash, eu_short_dot→dmy_short_dot, us_slash→mdy_slash, us_short_slash→mdy_short_slash
- Timestamps: american→mdy_12h, american_24h→mdy_24h, european→dmy_hm
- Numeric: decimal_number_eu→decimal_number_comma

Affects 13 files: 3 YAML, 8 Rust source, 2 CSV eval files.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All 10 type keys renamed in definitions_datetime.yaml and definitions_representation.yaml
- [x] #2 All generators updated in finetype-core/src/generator.rs
- [x] #3 label_category_map arrays updated in finetype-model/src/label_category_map.rs
- [x] #4 Disambiguation rules and header hints updated in finetype-model/src/column.rs
- [x] #5 DuckDB extension updated: type_mapping.rs, normalize.rs
- [x] #6 Eval infrastructure updated: matching.rs, data.rs, schema_mapping files
- [x] #7 cargo fmt && cargo clippy passes with no warnings
- [x] #8 cargo test passes (209 types)
- [x] #9 cargo run -- check passes (209/209)
- [x] #10 All 25 smoke tests pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Read key YAML files to confirm current state of type definitions
2. Rename 10 types in definitions_datetime.yaml and definitions_representation.yaml
3. Update all 13 occurrences in generator.rs (match arms)
4. Update label_category_map.rs (10 category array entries)
5. Update column.rs (12 occurrences in disambiguation + header hints)
6. Update DuckDB extension: type_mapping.rs (6) + normalize.rs (9)
7. Update eval files: matching.rs (1), data.rs (1), schema_mapping files (7 total)
8. Run cargo fmt && cargo clippy to fix formatting
9. Run cargo test to verify all 209 types pass
10. Run cargo run -- check to verify taxonomy alignment
11. Run bash tests/smoke.sh to verify 25 smoke tests
12. Add final summary with impact assessment
13. Commit with NNFT-234 in message
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Renamed 10 types from geographic names (eu_*/us_*/american/european) to format descriptors (dmy/mdy/comma). Affected:
- Dates: eu_slash→dmy_slash, eu_dot→dmy_dot, eu_short_slash→dmy_short_slash, eu_short_dot→dmy_short_dot, us_slash→mdy_slash, us_short_slash→mdy_short_slash
- Timestamps: american→mdy_12h, american_24h→mdy_24h, european→dmy_hm
- Numeric: decimal_number_eu→decimal_number_comma

Changes across 13 files: 3 YAML (definitions + eval mappings), 8 Rust (generators, models, DuckDB, eval), 2 CSV eval files.

Tests verify: 209/209 types passing, all 258 unit tests pass, 25/25 smoke tests pass, taxonomy check ✅. Alphabetical sorting fixed in label_category_map.rs.
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
