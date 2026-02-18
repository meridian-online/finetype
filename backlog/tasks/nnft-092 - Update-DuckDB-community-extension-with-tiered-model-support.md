---
id: NNFT-092
title: Update DuckDB community extension with tiered model support
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-17 22:44'
updated_date: '2026-02-18 00:48'
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
- [x] #1 finetype-core and finetype-model published to crates.io with tiered support
- [x] #2 duckdb-finetype repo updated with new crate versions
- [x] #3 Extension builds and passes SQL tests with tiered model
- [ ] #4 INSTALL finetype FROM community installs updated extension
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Copy tiered-v2 model from finetype repo to duckdb-finetype/models/
2. Update Cargo.toml: finetype-core/finetype-model 0.1.0 → 0.1.7
3. Rewrite build.rs to support tiered model embedding (detect tier_graph.json, generate TIER_GRAPH + get_tiered_model_data())
4. Update lib.rs: switch from CharClassifier to TieredClassifier via from_embedded()
5. Update type_mapping.rs: add any new types from 168-type taxonomy
6. Update SQL tests for tiered model outputs
7. Build and test locally
8. Commit and push
9. Update README.md with new type count
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1: finetype-core 0.1.7 + finetype-model 0.1.7 published to crates.io (NNFT-093)

AC#2: duckdb-finetype repo updated to v0.2.0. Changes:
- Cargo.toml: finetype-core/model 0.1.0 → 0.1.7, package version 0.2.0
- build.rs: Rewritten for tiered model embedding (detect tier_graph.json, 34 tier subdirs)
- lib.rs: TieredClassifier::from_embedded() replaces CharClassifier::from_bytes()
- unpack.rs: &CharClassifier → &dyn ValueClassifier
- normalize.rs: Added representation.boolean.* handling
- type_mapping.rs: 19 new type mappings (medical, payment, boolean, discrete, code, etc.)
- SQL tests: url instead of uri, representation.boolean.terms instead of technology.development.boolean
- Extension binary: 11MB (tiered model embedded)

AC#3: make release + make test_release both pass, cargo test passes (28 tests)

AC#4: PR #1328 created on duckdb/community-extensions to update ref and version.
Commit 0c79f5e pushed to noon-org/duckdb-finetype main."
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Updated DuckDB community extension from flat CharCNN (v0.1.0, 151 types) to tiered-v2 model (v0.2.0, 168 types).

## Extension Changes (noon-org/duckdb-finetype)
- **Model**: Replaced char-cnn-v2 (single flat model) with tiered-v2 (34 hierarchical CharCNN models)
- **build.rs**: Auto-detects tiered model via tier_graph.json, generates embedded lookup function for all tier subdirs
- **lib.rs**: Uses `TieredClassifier::from_embedded()` instead of `CharClassifier::from_bytes()`
- **unpack.rs**: Switched from concrete `&CharClassifier` to `&dyn ValueClassifier` trait object
- **normalize.rs**: Added `representation.boolean.*` normalization for new boolean types
- **type_mapping.rs**: Added 19 new DuckDB type mappings (medical, financial, boolean, discrete, code types)
- **SQL tests**: Updated expected labels for tiered model (url not uri, representation.boolean.terms not technology.development.boolean)
- **Extension binary**: 11MB with all 34 models embedded

## Distribution
- Commit 0c79f5e pushed to noon-org/duckdb-finetype main
- PR #1328 created on duckdb/community-extensions (updates ref + version + docs)
- Pending: CI build on community-extensions, then `INSTALL finetype FROM community` gets v0.2.0"
<!-- SECTION:FINAL_SUMMARY:END -->
