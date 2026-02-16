---
id: NNFT-080
title: Build automated profile-and-compare evaluation pipeline
status: To Do
assignee: []
created_date: '2026-02-16 10:49'
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
- [ ] #1 Pipeline accepts a directory of CSVs with ground truth annotations
- [ ] #2 Uses schema mapping to compare finetype predictions against GT
- [ ] #3 Produces per-type accuracy metrics (precision, recall, F1)
- [ ] #4 Separates model errors from semantic gaps in reporting
- [ ] #5 Outputs actionable list of misclassifications for investigation
- [ ] #6 Can re-run after model/rule changes to measure improvement
<!-- AC:END -->
