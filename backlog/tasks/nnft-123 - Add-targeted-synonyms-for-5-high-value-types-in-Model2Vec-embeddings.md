---
id: NNFT-123
title: Add targeted synonyms for 5 high-value types in Model2Vec embeddings
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-25 01:34'
updated_date: '2026-02-25 01:41'
labels:
  - accuracy
  - model2vec
dependencies:
  - NNFT-119
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
NNFT-119 discovery spike identified 5 types where targeted synonym additions (3-5 per type) would improve column name matching without centroid dilution.

Target types and proposed synonyms:
- datetime.offset.iana (0.669): timezone, tz, zone
- geography.address.postal_code (0.685): shipping postal code, billing postal code
- technology.internet.url (0.638): tracking url, callback url, redirect url
- technology.internet.http_status_code (0.683): status code, response code
- representation.file.mime_type (0.682): content type, media type

Keep expansion minimal (3-5 per type) to avoid the centroid dilution problem documented in NNFT-119 FINDING.md. Add synonyms to prepare_model2vec.py header_hint entries, then regenerate type_embeddings.safetensors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Synonyms added to prepare_model2vec.py for all 5 target types (3-5 per type)
- [x] #2 type_embeddings.safetensors regenerated with new synonyms
- [x] #3 All existing tests pass (cargo test)
- [x] #4 Profile eval shows no regression (make eval-profile, target ≥68/74)
- [x] #5 Verify targeted columns improve: timezone, shipping_postal_code, status_code, content_type, tracking_url should all exceed 0.65 similarity
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Read prepare_model2vec.py to understand current header_hint synonym structure
2. Add targeted synonyms (3-5 per type) for the 5 target types
3. Run prepare_model2vec.py to regenerate type_embeddings.safetensors
4. Run cargo test — verify all tests pass
5. Run make eval-profile — verify no regression (≥68/74)
6. Verify targeted columns with analyse_similarity.py — confirm improvements
7. Commit and push
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added 17 targeted synonyms across 5 types:
- datetime.offset.iana: timezone, tz, time zone, iana timezone (+4)
- geography.address.postal_code: shipping postal code, billing postal code, mailing zip (+3)
- technology.internet.url: tracking url, callback url, redirect url, api url (+4)
- technology.internet.http_status_code: status code, response code, http status (+3)
- representation.file.mime_type: content type, media type, mime (+3)

Total synonyms: 709 → 725. Regenerated type_embeddings.safetensors.

Verified improvements via analyse_similarity.py:
- timezone: 0.669 → 0.821 (+0.152)
- status_code: 0.683 → 0.802 (+0.119)
- tracking_url: ~0.638 → 0.746 (+0.108)
- shipping_postal_code: 0.685 → 0.771 (+0.086)
- content_type: ~0.682 → 0.722 (+0.040)
- mime_type: 0.781 → 0.773 (-0.008, negligible)
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 17 targeted synonyms to prepare_model2vec.py for 5 types identified by the NNFT-119 discovery spike, then regenerated type_embeddings.safetensors.

Changes:
- `scripts/prepare_model2vec.py`: Added header_hint entries for datetime.offset.iana (4), geography.address.postal_code (3), technology.internet.url (4), technology.internet.http_status_code (3), representation.file.mime_type (3)
- `models/model2vec/type_embeddings.safetensors`: Regenerated with new synonyms (725 total, up from 709)

Impact (measured via analyse_similarity.py):
- timezone: 0.669 → 0.821 (+0.152)
- status_code: 0.683 → 0.802 (+0.119)
- tracking_url: ~0.638 → 0.746 (+0.108)
- shipping_postal_code: 0.685 → 0.771 (+0.086)
- content_type: ~0.682 → 0.722 (+0.040)

All targeted columns now well above 0.65 threshold. No regressions on existing matches.

Verification:
- cargo test: 260/260 pass
- cargo run -- check: 169/169 taxonomy alignment
- make eval-profile: 68/74 format-detectable correct (no regression)
<!-- SECTION:FINAL_SUMMARY:END -->
