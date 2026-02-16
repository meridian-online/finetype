---
id: NNFT-081
title: Curate diverse benchmark dataset collection with type annotations
status: To Do
assignee: []
created_date: '2026-02-16 10:49'
labels:
  - evaluation
  - datasets
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Collect 20-30 classic/diverse CSV datasets with manually verified column type annotations as a regression test suite. Sources: Kaggle (Titanic, Housing, etc.), UCI ML Repository (Iris, Wine, Adult), data.gov, domain-specific datasets. Each dataset gets a ground truth YAML/JSON file mapping column names to expected finetype labels. This provides broader coverage than GitTables (which is GitHub-biased) and helps identify locale-specific patterns, taxonomy gaps, and new type opportunities. Focus on datasets that exercise different domains and locale formats.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 At least 20 diverse CSV datasets collected in eval/datasets/ or similar
- [ ] #2 Each dataset has a ground truth annotation file (column→expected finetype label)
- [ ] #3 Datasets span multiple domains: finance, healthcare, geography, technology, demographics
- [ ] #4 At least 3 datasets with non-English or locale-specific formats
- [ ] #5 Ground truth covers at least 50 distinct finetype types
- [ ] #6 Can be used as input to the profile-and-compare pipeline
<!-- AC:END -->
