---
id: NNFT-120
title: 'Release v0.2.0: attractor demotion, JSON Schema validation, numeric ranges'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-24 11:54'
updated_date: '2026-02-24 12:09'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release FineType v0.2.0 with three significant changes since v0.1.9:

1. **NNFT-115** — Multi-signal attractor demotion (Rule 14). Demotes over-eager specific type predictions using validation, confidence, and cardinality signals. 17 predictions improved, 0 regressions.
2. **NNFT-116** — JSON Schema validator migration. Replaced hand-rolled regex with jsonschema-rs (v0.42.1). CompiledValidator pre-compiles schemas once; hybrid strategy for string/numeric validation.
3. **NNFT-117** — Numeric range validation. Added maximum: 99999 constraint to postal_code and street_number schemas, eliminating false positives on salary, ticket, and byte count columns.

Also backfills missing v0.1.9 CHANGELOG entry.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cargo.toml workspace version bumped to 0.2.0
- [x] #2 CHANGELOG.md has v0.2.0 section with NNFT-115, NNFT-116, NNFT-117
- [x] #3 CHANGELOG.md has backfilled v0.1.9 section with NNFT-109, NNFT-110
- [x] #4 CLAUDE.md version updated to 0.2.0 with milestone entry
- [x] #5 cargo build succeeds
- [x] #6 cargo test passes (249 tests)
- [x] #7 Tagged v0.2.0 and pushed to origin
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released FineType v0.2.0 with three changes since v0.1.9:

**Accuracy:**
- NNFT-115: Multi-signal attractor demotion (Rule 14) — demotes over-eager predictions using validation, confidence, and cardinality signals. 17 predictions improved, 0 regressions.
- NNFT-117: Numeric range validation — maximum: 99999 constraint on postal_code and street_number eliminates false positives on salary, ticket, and byte count columns.

**Changed:**
- NNFT-116: JSON Schema validation engine — migrated from hand-rolled regex to jsonschema-rs v0.42.1. CompiledValidator pre-compiles schemas; hybrid strategy for string/numeric validation.

**Housekeeping:**
- Backfilled missing v0.1.9 CHANGELOG entry (NNFT-109 unified disambiguation, NNFT-110 Model2Vec)
- Updated CLAUDE.md: version, milestones, moved NNFT-115/116 from in-progress to v0.2.0 milestone

**CI fix (follow-up commit):**
- Fixed 7 taxonomy enum mismatches exposed by NNFT-116's stricter JSON Schema validation:
  - year: minimum 1900 → 1000 (generator produces historical years)
  - street_suffix: added "Place"
  - degree: added specific degree names (Bachelor of Science, Master of Arts, etc.)
  - programming_language: natural casing + added Perl, Lua
  - software_license: SPDX casing + added CC0-1.0
  - stage: natural casing + added LTS
  - os: added Linux distros + versioned Windows
- Taxonomy check now 169/169 passing, 8450/8450 samples (100.0%)

**Verification:**
- cargo build: pass
- cargo test: 249 tests pass (91 core + 158 model)
- cargo run -- check: 169/169 fully passing (was 162/169)
- Pre-commit hooks: fmt, clippy, test all pass
- Tagged v0.2.0 (moved to include enum fix), pushed to origin

**Post-CI TODO:** Publish to crates.io after CI builds complete.
<!-- SECTION:FINAL_SUMMARY:END -->
