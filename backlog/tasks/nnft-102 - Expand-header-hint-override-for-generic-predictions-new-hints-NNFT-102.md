---
id: NNFT-102
title: Expand header hint override for generic predictions + new hints (NNFT-102)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 07:20'
updated_date: '2026-02-18 07:21'
labels:
  - accuracy
  - disambiguation
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Profile eval accuracy was 85.8% (97/113). Analysis of 16 remaining format-detectable errors showed 3 actionable patterns that could be fixed with header hint improvements:

1. UTC offset (integer_number@0.8) - header hint existed but couldn't fire because hinted type not in votes
2. Alpha-3 country codes (iata_code@0.5) - no header hint for "alpha-3" column names
3. OS names (phone_number@0.412) - header hint existed but phone_number not in is_generic

Root cause for #1 and #3: the header hint mechanism required the hinted type to appear in the model's vote distribution (hint_in_votes). When the model prediction was generic but the hinted type wasn't a candidate, the hint was silently ignored.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Header hints override generic predictions even when hinted type not in vote distribution
- [x] #2 alpha-2/alpha-3 column names map to country_code
- [x] #3 phone_number and iata_code added to is_generic list
- [x] #4 Occupation/job_title header hints added
- [x] #5 Profile eval accuracy improves from 85.8% (no regressions on previously correct columns)
- [x] #6 All tests pass, fmt/clippy clean
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Profile eval accuracy improved from 85.8% (97/113) to **92.9% (105/113)**, a +7.1pp lift fixing 8 columns.

## Key change: header_hint_generic override path

Previously, header hints required the hinted type to appear in the model's vote distribution (`hint_in_votes`). This silently ignored hints when the model prediction was generic (integer_number, username, phone_number, etc.) but the hinted type wasn't a model candidate.

New logic: when `is_generic=true` AND a header hint exists, apply the hint at confidence=0.5 regardless of vote distribution. This trusts the column name over a generic model prediction.

## Changes
- `header_hint_generic` override path in `classify_column_with_header()`
- `phone_number` and `iata_code` added to `is_generic` list
- `alpha-2`, `alpha-3`, `iso alpha 2/3` → `country_code` header hints
- `occupation`, `job_title`, `profession`, `role`, `position` header hints
- Split `country` vs `country code` header hints (were merged into one)

## Columns fixed
airports.utc_offset, countries.alpha-3, tech_systems.os, people_directory.job_title, covid_timeseries.Country, people_directory.weight_kg, medical_records.weight_lbs, people_directory.height_cm

## Remaining 8 errors (structural)
Name ambiguity (5): city/country/publisher/company/hostname confused with person names
Financial codes (2): CVV vs postal_code, SWIFT vs SEDOL — near-identical formats
Height (1): height_in predicted as age at 0.967 confidence

Commit d8c125a pushed to main."
<!-- SECTION:FINAL_SUMMARY:END -->
