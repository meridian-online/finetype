---
id: NNFT-092
title: Update DuckDB community extension with tiered model support
status: To Do
assignee: []
created_date: '2026-02-17 22:44'
labels:
  - release
  - duckdb
dependencies:
  - NNFT-021
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The noon-org/duckdb-finetype extension currently uses the flat CharCNN model via finetype-core and finetype-model from crates.io. Once the tiered model is the default, we need to:

1. Publish updated finetype-core and finetype-model crates to crates.io
2. Update duckdb-finetype Cargo.toml to use new crate versions
3. Verify the extension embeds tiered model and uses tiered inference
4. The community extensions CI should automatically rebuild on next cycle

No new PR to duckdb/community-extensions needed — the CI rebuilds from the noon-org/duckdb-finetype repo.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 finetype-core and finetype-model published to crates.io with tiered support
- [ ] #2 duckdb-finetype repo updated with new crate versions
- [ ] #3 Extension builds and passes SQL tests with tiered model
- [ ] #4 INSTALL finetype FROM community installs updated extension
<!-- AC:END -->
