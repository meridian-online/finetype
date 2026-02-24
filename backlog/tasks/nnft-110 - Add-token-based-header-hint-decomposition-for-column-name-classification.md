---
id: NNFT-110
title: Add token-based header hint decomposition for column name classification
status: To Do
assignee:
  - '@nightingale'
created_date: '2026-02-18 11:04'
labels:
  - accuracy
  - feature
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace the current exact-match header hint dictionary with a token-decomposition approach that splits column names on separators (_, camelCase, spaces) and looks up each token in a weighted signal dictionary.

Example: "invoice_created_at" → tokens ["invoice", "created", "at"] → signals {created: datetime 0.8, at: datetime 0.3} → datetime domain.

Also add prefix/suffix patterns: ts_, _ts, _dt → datetime; is_, has_, _flag → boolean; _id, _key → identifier; _pct, _rate → numeric; _lat, _lng → coordinate; _addr → address.

This captures 80% of transformer-level benefit at near-zero cost.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Column names tokenized by splitting on _, camelCase boundaries, spaces, and dots
- [ ] #2 Weighted token signal dictionary with domain/type affinities
- [ ] #3 Prefix/suffix pattern table for common abbreviations (ts_, _id, _dt, is_, etc.)
- [ ] #4 Token signals combined via weighted vote to produce header-based type suggestion
- [ ] #5 Header suggestion used as tiebreaker when CharCNN vote is ambiguous
- [ ] #6 Existing header hint exact matches preserved as highest priority
- [ ] #7 Unit tests for tokenization and signal aggregation
<!-- AC:END -->
