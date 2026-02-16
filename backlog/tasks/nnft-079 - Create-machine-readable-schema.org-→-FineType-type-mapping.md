---
id: NNFT-079
title: Create machine-readable schema.org → FineType type mapping
status: To Do
assignee: []
created_date: '2026-02-16 10:49'
labels:
  - evaluation
  - taxonomy
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Codify the schema.org/DBpedia → FineType label mapping from TAXONOMY_COMPARISON.md into a machine-readable format (YAML or CSV at eval/schema_mapping.csv). Each row maps a ground truth label to a finetype label with match quality (direct/close/partial/semantic_only). This enables automated evaluation scoring. Also useful for identifying taxonomy growth opportunities — GT labels with reproducible patterns that finetype doesn't yet cover.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Mapping file exists at eval/schema_mapping.csv or eval/schema_mapping.yaml
- [ ] #2 Covers all ~180 unique GT labels from GitTables 1M metadata
- [ ] #3 Each entry has: gt_label, finetype_label (or NULL), match_quality (direct/close/partial/semantic_only)
- [ ] #4 Format-detectable types have correct finetype mappings
- [ ] #5 Semantic-only types explicitly marked as NULL/unmappable
- [ ] #6 Boundary types flagged for potential taxonomy expansion
<!-- AC:END -->
