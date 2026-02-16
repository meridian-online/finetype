---
id: NNFT-079
title: Create machine-readable schema.org → FineType type mapping
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-16 10:49'
updated_date: '2026-02-16 10:59'
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
- [x] #1 Mapping file exists at eval/schema_mapping.csv or eval/schema_mapping.yaml
- [x] #2 Covers all ~180 unique GT labels from GitTables 1M metadata
- [x] #3 Each entry has: gt_label, finetype_label (or NULL), match_quality (direct/close/partial/semantic_only)
- [x] #4 Format-detectable types have correct finetype mappings
- [x] #5 Semantic-only types explicitly marked as NULL/unmappable
- [x] #6 Boundary types flagged for potential taxonomy expansion
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Collect all unique GT labels from schema_labels.csv (59) + dbpedia_labels.csv (122) = 181 unique labels
2. Cross-reference with TAXONOMY_COMPARISON.md prose mappings and eval_1m.sql type_mapping table
3. Map each GT label to best FineType label (or NULL), classify match quality
4. Also query actual GT label frequencies from 1M metadata for prioritization
5. Create eval/schema_mapping.yaml with structured mappings
6. Validate mapping covers all labels, run through a quick sanity check
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Created eval/schema_mapping.yaml with 191 entries covering all 145 unique GT labels from schema_labels.csv (59) + dbpedia_labels.csv (122), plus 46 additional long-tail labels from the 1M corpus metadata.

Validation results:
- All finetype_label references valid against 169-type taxonomy
- All finetype_domain values valid
- 17 direct matches, 21 close, 33 partial, 120 semantic_only
- 11 expansion candidates flagged (ticker symbol, filename, citation, language code, formula, etc.)

The mapping uses YAML for richer structure: each entry has gt_label, source, finetype_label, finetype_domain, match_quality, expand flag, and notes.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Created eval/schema_mapping.yaml — a machine-readable mapping of schema.org and DBpedia ground truth labels to FineType types for automated evaluation scoring.

Coverage:
- 191 entries covering all 145 unique GT labels from schema_labels.csv (59) and dbpedia_labels.csv (122)
- 46 additional long-tail labels from the GitTables 1M corpus metadata
- All finetype_label references validated against the 169-type taxonomy

Match quality breakdown:
- 17 direct matches (email, url, gender, postal_code, country, city, state, issn, etc.)
- 21 close matches (name→full_name, author→full_name, address→full_address, etc.)
- 33 partial matches (date→datetime.*, id→increment, code→alphanumeric_id, etc.)
- 120 semantic-only (title, description, category, rating, price — no format signal)

11 taxonomy expansion candidates flagged:
- ticker symbol, filename, citation, language code, ISO code, data type, formula, organization name, serial number, order number, language

Schema per entry: gt_label, source, finetype_label, finetype_domain, match_quality, expand, notes

This mapping is the prerequisite for NNFT-080 (automated eval pipeline) and NNFT-082 (GitTables v6 eval).
<!-- SECTION:FINAL_SUMMARY:END -->
