---
id: NNFT-120
title: 'Release v0.2.0: attractor demotion, JSON Schema validation, numeric ranges'
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-24 11:54'
updated_date: '2026-02-24 11:54'
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
- [ ] #7 Tagged v0.2.0 and pushed to origin
<!-- AC:END -->
