---
id: NNFT-258
title: >-
  Expand golden tests — structured CLI regression suite for profile, load,
  taxonomy, schema
status: To Do
assignee: []
created_date: '2026-03-08 06:45'
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
- [ ] #1 Rust integration test suite exists under tests/ covering profile, load, taxonomy, and schema commands
- [ ] #2 Profile tests use structured field matching — assert type label and broad_type per column, not byte-for-byte output comparison
- [ ] #3 At least 4 real-world CSV datasets have profile golden tests (datetime_formats, ecommerce_orders, titanic, people_directory)
- [ ] #4 At least 3 small focused fixture CSVs test edge cases (ambiguous headers, numeric-only columns, single-char categoricals)
- [ ] #5 Load tests verify the generated DuckDB DDL contains correct column types (DATE, TIMESTAMP, BIGINT etc. — not VARCHAR for typed columns)
- [ ] #6 Taxonomy and schema command tests verify output structure and key fields
- [ ] #7 Tests gated appropriately (feature flag or #[ignore]) so cargo test stays fast for dev workflow
- [ ] #8 All tests pass in CI
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
