---
id: NNFT-122
title: Lower Model2Vec semantic hint threshold from 0.70 to 0.65
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-25 01:34'
updated_date: '2026-02-25 01:37'
labels:
  - accuracy
  - model2vec
dependencies:
  - NNFT-119
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
NNFT-119 discovery spike found that lowering the Model2Vec cosine similarity threshold from 0.70 to 0.65 is the highest-value, lowest-risk change for improving column name classification.

Expected impact (from discovery data):
- +12 true positives recovered (timezone, ean, shipping_postal_code, status_code, content_type, price variants, tracking_url, alpha-2/3, rating, unix_ms)
- +2 false positives on eval columns (borderline cases)
- +1 false positive on generic names (data → form_data at 0.687)
- Precision: 93.1% (vs current 94.1%)
- Recall: 74.0% (vs current 65.8%)

Single constant change in semantic.rs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 DEFAULT_THRESHOLD constant in semantic.rs changed from 0.70 to 0.65
- [x] #2 All existing tests pass (cargo test)
- [x] #3 Profile eval shows no regression (make eval-profile, target ≥68/74)
- [x] #4 Taxonomy check passes (cargo run -- check, 169/169)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Change DEFAULT_THRESHOLD from 0.70 to 0.65 in semantic.rs
2. Run cargo test — verify all existing tests pass
3. Run cargo run -- check — verify 169/169 taxonomy alignment
4. Run make eval-profile — verify ≥68/74 (no regression from 68/74 baseline)
5. Commit and push
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Changed DEFAULT_THRESHOLD from 0.70 to 0.65 in semantic.rs. Updated doc comment to reference FINDING.md data. Updated integration test: removed 'data' from generic rejection list and added explicit assertion that data→form_data is the expected borderline match at 0.65. Updated CLAUDE.md threshold references (architecture section + decided item 5a).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Lowered Model2Vec semantic hint threshold from 0.70 to 0.65, recovering 12 additional correct column name matches with minimal false positive risk.

Changes:
- `semantic.rs`: DEFAULT_THRESHOLD 0.70 → 0.65, updated doc comment
- `semantic.rs`: Updated integration test — 'data' is now a known borderline match (0.687 → form_data), not a generic rejection
- `CLAUDE.md`: Updated threshold references in architecture and decided items sections

Verification:
- cargo test: 260/260 pass
- cargo run -- check: 169/169 taxonomy alignment
- make eval-profile: 68/74 format-detectable correct (no regression)
<!-- SECTION:FINAL_SUMMARY:END -->
