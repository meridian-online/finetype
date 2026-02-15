---
id: NNFT-056
title: Expand address training data to more locales
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-14 10:08'
updated_date: '2026-02-15 09:07'
labels:
  - generator
  - locale
  - data-quality
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Improve address training data coverage across locales. Currently geography.address types are locale-specific but training data may be biased toward EN formats.

Address format conventions vary significantly:
- US/UK: number + street + city + state + zip
- Japan: prefecture + city + district + block (large to small)
- Germany: street + number + PLZ + city
- France: number + street + code postal + ville

Better locale coverage will improve real-world accuracy on international datasets. Could use CLDR address format data or locale-specific faker libraries as data sources.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Address generators produce locale-accurate formatting for at least 10 locales
- [x] #2 Street name, suffix, and number formats match locale conventions
- [x] #3 Full address format ordering matches locale expectations (e.g., JP large-to-small)
- [x] #4 Training data balanced across locales to avoid EN bias
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Rewrite full_address generator with locale-specific format templates:
   - EN/US/CA: {num} {street}, {city}, {state} {zip}
   - EN_GB: {num} {street}, {city} {postcode}
   - EN_AU: {num} {street}, {city} {state} {postcode}
   - DE: {street} {num}, {plz} {city}
   - FR: {num} {street}, {code_postal} {city}
   - ES: {street} {num}, {cp} {city}
   - IT: {street} {num}, {cap} {city}
   - NL: {street} {num}, {postcode} {city}
   - PL: {street} {num}, {code} {city}
   - RU: {city}, {street}, д. {num}
   - JA: {prefecture}{city}{district}{block}{num}
   - ZH: {province}{city}{district}{street}{num}号
   - KO: {city} {district} {street} {num}
   - AR: {street} {num}, {city}
2. Add states/provinces/regions to locale_data for US, AU, DE, JP, CN, KO
3. Run finetype check and cargo test
4. Verify generated addresses show locale diversity
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rewrote full_address generator with locale-specific format templates for all 16 locales (14 unique formats). Added states_or_regions() and districts() functions to locale_data.rs.

Format ordering per locale:
- EN/US/CA: {num} {street}, {city}, {state} {zip} (with optional Apt)
- EN_GB: {num} {street}, {city}, {postcode}
- EN_AU: {num} {street}, {city} {state} {postcode}
- DE: {street} {num}, {PLZ} {city}
- FR: {num} {street}, {CP} {city}
- ES/IT: {street} {num}, {CAP} {city}
- NL: {street} {num}, {postcode} {city}
- PL: {street} {num}, {code} {city}
- RU: {city}, {street}, д. {num}, {index}
- JA: 〒{postal} {prefecture}{district}{chome}-{ban}-{go}
- ZH: {province}{city}{district}{street}{num}号
- KO: ({postal}) {city} {district} {street} {num}
- AR: {street} {num}، {city}

Added locale_data:
- states_or_regions(): US states (50), CA provinces (13), AU states (8), DE Bundesländer, FR régions, ES comunidades, IT regioni, JP prefectures, CN provinces, KO cities, RU cities
- districts(): JA 区 (10), ZH 区 (10), KO 구 (10)

168 types, 8400/8400 samples pass, 169 tests pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded full_address generator with locale-specific format templates for all 16 locales.

Changes:
- Rewrote full_address to dispatch to gen_full_address() with locale-aware formatting
- 14 distinct format templates matching real-world address conventions per locale
- EN: num+street+city+state+zip, DE: street+num+PLZ+city, JA: postal+prefecture+district+chome, etc.
- Added states_or_regions() to locale_data: US (50 states), CA (13 provinces), AU (8 states), plus DE/FR/ES/IT/JP/ZH/KO/RU/AR regions
- Added districts() to locale_data: JA/ZH/KO district names for East Asian address formatting
- Training data now balanced: each locale gets equal samples with format-accurate output

Taxonomy: 168 types, 8400/8400 samples pass, 169 tests pass.
<!-- SECTION:FINAL_SUMMARY:END -->
