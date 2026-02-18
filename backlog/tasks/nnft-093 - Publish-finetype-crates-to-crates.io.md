---
id: NNFT-093
title: Publish finetype crates to crates.io
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-17 22:44'
updated_date: '2026-02-18 00:19'
labels:
  - release
  - infrastructure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
finetype-core and finetype-model need to be published to crates.io with the latest changes (ValueClassifier trait, tiered inference support). This is a prerequisite for updating the DuckDB community extension, which depends on these crates.

Need to check if crates are already published and what version they're at, then publish updates with the tiered model support.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 finetype-core published to crates.io with current changes
- [x] #2 finetype-model published to crates.io with ValueClassifier trait and tiered support
- [x] #3 Version numbers consistent with workspace Cargo.toml
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Published finetype-core v0.1.7 and finetype-model v0.1.7 to crates.io.

Changes:
- Updated workspace dependency version specs from "0.1.2" to "0.1.7" in root Cargo.toml
- Published finetype-core first (11 files, 368KB), then finetype-model (13 files, 279KB)
- finetype-model correctly resolves finetype-core v0.1.7 from crates.io registry

Note: finetype-cli (v0.1.0 on crates.io) not updated — its embed-models default feature requires model files at build time which aren't available from crates.io. CLI distribution is via GitHub releases and Homebrew.

Commit: af7161a (version bump)
crates.io: finetype-core 0.1.7, finetype-model 0.1.7
<!-- SECTION:FINAL_SUMMARY:END -->
