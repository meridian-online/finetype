---
id: NNFT-114
title: Publish finetype-core and finetype-model 0.1.9 to crates.io
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-24 05:49'
updated_date: '2026-02-24 05:49'
labels:
  - release
  - distribution
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Publish updated crates to crates.io following v0.1.9 release. Previous version on crates.io was 0.1.7.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 finetype-core 0.1.9 published to crates.io
- [x] #2 finetype-model 0.1.9 published to crates.io
- [x] #3 Verify finetype-model resolves finetype-core 0.1.9 dependency
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Published finetype-core 0.1.9 and finetype-model 0.1.9 to crates.io (previous: 0.1.7).

Published in dependency order — core first, then model. Both verified during publish (cargo builds from packaged crate successfully). finetype-model correctly resolved finetype-core 0.1.9 from the registry.

Package sizes: core 73.8KB compressed (11 files), model 69.3KB compressed (14 files).
<!-- SECTION:FINAL_SUMMARY:END -->
