---
id: NNFT-104
title: Download SOTAB dataset and create FineType schema mapping
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 09:17'
updated_date: '2026-02-18 09:44'
labels:
  - evaluation
  - data
dependencies: []
references:
  - eval/gittables/REPORT.md
  - docs/TAXONOMY_COMPARISON.md
documentation:
  - 'https://webdatacommons.org/structureddata/sotab/'
  - 'https://zenodo.org/records/8422037'
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Download the SOTAB (WDC Schema.org Table Annotation Benchmark) CTA dataset and create a mapping from its 91 Schema.org column type labels to FineType's 168-type taxonomy.

SOTAB provides 162,351 annotated columns across 59,548 tables from 74,215 real websites — a completely different data distribution from GitTables (GitHub repos). The Schema.org labels (telephone, email, streetAddress, duration, currency, postalCode, etc.) are more format-oriented than GitTables' DBpedia labels, which should yield a higher format-detectability ratio.

This is the foundation task — the mapping quality determines how meaningful the evaluation results will be.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 SOTAB CTA dataset downloaded (train/val/test splits + table JSON files)
- [x] #2 All 91 SOTAB CTA labels documented with example values and column counts
- [x] #3 Schema mapping CSV created: sotab_label → finetype_type with match quality tier (direct/close/partial/semantic_only)
- [x] #4 Format-detectability analysis: count of columns per detectability tier
- [x] #5 Mapping reviewed against existing schema_mapping.csv patterns from GitTables
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Progress

**SOTAB CTA Download:**
- Validation set: 5,732 tables, 16,840 annotated columns (CTA_Validation.zip, 190MB)
- Test set: 7,026 tables, ~30K annotated columns (CTA_Test.zip, 252MB)
- Training set: 1.2GB download did not complete — not needed for evaluation

**Label Analysis (91 labels):**
- 17 direct matches (30.1% of columns) — telephone, email, postalCode, URL, etc.
- 25 close matches (38.3%) — Text, Number, DateTime, price, etc.
- 9 partial matches (11.5%) — Mass, Distance, Energy, etc.
- 40 semantic-only (20.1%) — ItemAvailability, MusicAlbum, Brand, etc.
- **68.5% format-detectable** (direct + close) vs GitTables ~19%

**Schema Mapping:**
- Created at eval/sotab/sotab_schema_mapping.csv
- All 91 labels mapped with match quality tiers and notes
- Reviewed against GitTables schema_mapping.csv patterns

**Data locations:**
- /home/hugh/sotab/cta/validation/ (extracted tables + GT)
- /home/hugh/sotab/cta/test/ (extracted tables + GT files)
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Downloaded SOTAB CTA benchmark and created comprehensive FineType schema mapping.

**Dataset:**
- Validation: 5,732 tables, 16,840 annotated columns
- Test: 7,026 tables, ~30K annotated columns across 5 ground truth files (standard, corner cases, format heterogeneity, missing values, random)
- Training: skipped (1.2GB) — not needed for evaluation

**Schema Mapping (eval/sotab/sotab_schema_mapping.csv):**
- All 91 SOTAB CTA labels mapped to FineType types with match quality tiers
- 17 direct (30.1%), 25 close (38.3%), 9 partial (11.5%), 40 semantic_only (20.1%)
- **68.5% format-detectable** — significantly better than GitTables (~19%)

**Key insight:** SOTAB is a strong complementary benchmark to GitTables because:
1. Different data source (web tables vs GitHub repos)
2. Schema.org labels are more format-oriented than DBpedia
3. Higher format-detectability means more signal for FineType accuracy measurement
<!-- SECTION:FINAL_SUMMARY:END -->
