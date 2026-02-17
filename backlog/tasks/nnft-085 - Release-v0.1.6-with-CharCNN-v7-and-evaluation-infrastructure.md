---
id: NNFT-085
title: Release v0.1.6 with CharCNN v7 and evaluation infrastructure
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-17 06:55'
updated_date: '2026-02-17 06:57'
labels:
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Release v0.1.6 containing:
- NNFT-083: Critical training bug fix (locale suffix mapping), generator improvements, CharCNN v7 (first correctly-trained model)
- NNFT-079: Schema mapping (schema.org/DBpedia → FineType)
- NNFT-080: Automated profile evaluation pipeline
- NNFT-081: 20 benchmark datasets with 206 ground truth annotations
- NNFT-082: GitTables 1M evaluation

This is a critical release because v0.1.5 shipped a broken model (always predicted comma_separated due to training label mapping bug).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Version bumped to 0.1.6 in all Cargo.toml files
- [x] #2 Git tag v0.1.6 created and pushed
- [x] #3 All tests pass
- [x] #4 Release notes document the training bug fix as breaking change from v0.1.5
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Released v0.1.6 (tag 348c2b1). Critical fix: training label mapping bug that caused v0.1.3–v0.1.5 models to always predict class 0. CharCNN v7 is the first correctly-trained model (85.14% accuracy). Includes evaluation infrastructure (NNFT-079–082) and generator improvements (NNFT-083)."
<!-- SECTION:FINAL_SUMMARY:END -->
