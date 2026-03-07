---
id: NNFT-244.01
title: Taxonomy expansion — geography domain (+10 types)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 05:10'
updated_date: '2026-03-07 05:53'
labels:
  - taxonomy
  - expansion
  - geography
dependencies: []
references:
  - discovery/taxonomy-revision/EXPANSION.md
  - labels/definitions_geography.yaml
parent_task_id: NNFT-244
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add ~8 new geography types from EXPANSION.md Tiers 1-4:

**New categories:** format (wkt, geojson), index (h3), transportation (iso6346, hs_code, unlocode)
**New coordinate types:** geohash, plus_code, dms, mgrs

Each type needs: YAML definition (validation, format_string, transform, broad_type, tier), generator, and transform_ext where DuckDB extensions exist (spatial for WKT/GeoJSON, h3 for H3).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 YAML definitions added for all geography types with validation, format_string, transform, broad_type, tier
- [x] #2 Generators produce valid samples that pass validation for each new type
- [x] #3 `finetype check` passes with all new geography types
- [x] #4 `finetype schema` exports valid JSON Schema for each new type
- [x] #5 transform_ext fields populated for WKT (spatial), GeoJSON (spatial), H3 (h3 extension)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 10 new geography types across 3 new categories (format, index, transportation) and 1 existing category (coordinate):

**New types:**
- `geography.format.wkt` — Well-Known Text geometry (transform_ext: spatial)
- `geography.format.geojson` — GeoJSON geometry objects (transform_ext: spatial)
- `geography.index.h3` — H3 hexagonal cell index (transform_ext: h3)
- `geography.index.geohash` — Geohash encoded coordinates
- `geography.coordinate.plus_code` — Google Plus Codes (Open Location Code)
- `geography.coordinate.dms` — Degrees/Minutes/Seconds coordinates
- `geography.coordinate.mgrs` — Military Grid Reference System
- `geography.transportation.iso6346` — Container codes (ISO 6346)
- `geography.transportation.hs_code` — Harmonized System commodity codes
- `geography.transportation.unlocode` — UN/LOCODE location codes

All types have validation regex, generators, format_string, transform, broad_type, and tier fields. transform_ext populated for WKT, GeoJSON (spatial), and H3 (h3 extension). LabelCategoryMap updated with GEOGRAPHIC_LABELS entries.

Commit: 6f58d45
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
