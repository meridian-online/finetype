---
id: NNFT-093
title: Publish finetype crates to crates.io
status: To Do
assignee: []
created_date: '2026-02-17 22:44'
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
- [ ] #1 finetype-core published to crates.io with current changes
- [ ] #2 finetype-model published to crates.io with ValueClassifier trait and tiered support
- [ ] #3 Version numbers consistent with workspace Cargo.toml
<!-- AC:END -->
