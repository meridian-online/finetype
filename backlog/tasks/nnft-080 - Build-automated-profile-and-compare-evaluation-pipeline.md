---
id: NNFT-080
title: Build automated profile-and-compare evaluation pipeline
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-16 10:49'
updated_date: '2026-02-16 21:58'
labels:
  - evaluation
  - tooling
dependencies:
  - NNFT-079
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create a pipeline that runs finetype profile on a directory of annotated CSVs, compares predictions against ground truth using the schema mapping, and produces structured accuracy reports. Should distinguish model errors (finetype should detect but didn't) from format gaps (semantic-only labels). Could be a DuckDB SQL script, a new CLI subcommand, or a Makefile target. Key output: per-type precision/recall, confusion matrix for mappable types, list of misclassifications for investigation. This scales the Titanic-style iteration loop to N datasets.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pipeline accepts a directory of CSVs with ground truth annotations
- [x] #2 Uses schema mapping to compare finetype predictions against GT
- [x] #3 Produces per-type accuracy metrics (precision, recall, F1)
- [x] #4 Separates model errors from semantic gaps in reporting
- [x] #5 Outputs actionable list of misclassifications for investigation
- [x] #6 Can re-run after model/rule changes to measure improvement
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Built a three-phase automated evaluation pipeline for comparing finetype profile predictions against ground truth annotations.

Pipeline components:
- **eval/profile_eval.sh** — Orchestration script that profiles CSVs from a manifest, extracts GT annotations, and feeds results to DuckDB
- **eval/eval_profile.sql** — DuckDB analysis script with 9 report sections: data loading, overall accuracy, per-type metrics (precision/recall), model errors vs semantic gaps, confusion matrix, actionable misclassifications, expansion opportunities, per-dataset summary
- **eval/datasets/manifest.csv** — Sample Titanic manifest with 12 column annotations
- **eval/schema_mapping.csv** — Flat CSV generated from YAML mapping for DuckDB
- **Makefile** — `make eval-profile` target (with optional MANIFEST= override)

Key design decisions:
- Manifest format (dataset, file_path, column_name, gt_label) allows multiple datasets in one evaluation run
- Three-tier detectability classification: format_detectable (direct+close), partially_detectable (partial), semantic_only — separates model errors from design gaps
- Label-level AND domain-level accuracy reported separately
- Python CSV writer `\r\n` line endings stripped with `tr -d '\r'` for DuckDB compatibility
- Added `boolean` GT label to schema mapping (NNFT-079 update)

Titanic baseline results:
- Format-detectable label accuracy: 100% (4/4: gender, boolean, age, name)
- Domain accuracy: 91.7% (11/12 columns)
- Actionable misclassifications: PassengerId→cvv (should be increment), Ticket→wrong alphanumeric path
<!-- SECTION:FINAL_SUMMARY:END -->
