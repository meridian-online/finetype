---
id: NNFT-157
title: >-
  CLDR data acquisition and pattern mapping (Phase 1 of CLDR-enriched
  retraining)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-27 22:48'
updated_date: '2026-02-27 22:53'
labels:
  - accuracy
  - cldr
  - phase-1
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 1 of the CLDR-Enriched Model Retraining plan (Option C).

Download CLDR JSON packages (cldr-dates-full, cldr-numbers-full) and build extraction scripts that map LDML date/time patterns to FineType datetime types. The mapping table enables Phase 2 generator enrichment.

Key deliverables:
- scripts/download_cldr.sh — fetches CLDR JSON packages to data/cldr/
- scripts/extract_cldr_patterns.py — maps LDML patterns → FineType types, outputs mapping report
- .gitignore updated for CLDR JSON data
- data/cldr/README.md updated with CLDR date/time format source documentation

CJK date formats (y年M月d日) are explicitly excluded — they require new taxonomy types.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 download_cldr.sh fetches cldr-dates-full and cldr-numbers-full to data/cldr/ with version pinning
- [x] #2 extract_cldr_patterns.py produces mapping table of LDML patterns → FineType datetime types
- [x] #3 Mapping covers all 4 CLDR format lengths (short/medium/long/full) for 20+ locales
- [x] #4 Month and weekday names extracted for 20+ locales (wide + abbreviated)
- [x] #5 CJK formats explicitly excluded with documented rationale
- [x] #6 .gitignore updated to exclude CLDR JSON data directories
- [x] #7 data/cldr/README.md updated with CLDR date/time format source documentation
- [x] #8 Both scripts run successfully and produce valid output
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Create scripts/download_cldr.sh — downloads cldr-dates-full and cldr-numbers-full npm packages (v48.0.0) to data/cldr/json/
2. Update .gitignore to exclude data/cldr/json/ (CLDR raw data, not committed)
3. Create scripts/extract_cldr_patterns.py — parses CLDR JSON, maps LDML patterns to FineType types
   - Walk all locale ca-gregorian.json files
   - Extract dateFormats (short/medium/long/full) and timeFormats
   - Extract months (wide + abbreviated) and days (wide + abbreviated) 
   - Map each LDML pattern shape to FineType datetime type
   - Exclude CJK locales (ja, zh, ko) with documented rationale
   - Output mapping report as TSV + summary stats
4. Run both scripts, verify output
5. Update data/cldr/README.md with CLDR date/time format source documentation
6. Run cargo test + cargo run -- check to verify no regressions
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
CLDR data acquisition and pattern mapping for training data enrichment (Phase 1 of 5).

## What changed

Added infrastructure to download and extract Unicode CLDR date/time patterns for enriching FineType training data. This is the foundation for CLDR-enriched model retraining.

### New files

- **scripts/download_cldr.sh** — Downloads `cldr-dates-full`, `cldr-numbers-full`, and `cldr-core` npm packages (v46.0.0) to `data/cldr/json/`. Version-pinned with manifest.
- **scripts/extract_cldr_patterns.py** — Parses all CLDR locale JSON files, maps LDML date/time patterns to FineType types, extracts month and weekday names (wide + abbreviated). Outputs 5 files: date patterns TSV, time patterns TSV, month names TSV, weekday names TSV, mapping report.

### Modified files

- **.gitignore** — Added `data/cldr/json/` exclusion (raw CLDR JSON is ~15MB, not committed)
- **data/cldr/README.md** — Added comprehensive documentation for CLDR date/time format patterns, LDML→FineType mapping table, CJK exclusion rationale, and script usage.

### Extracted data (committed)

- `data/cldr/cldr_date_patterns.tsv` — 2823 date patterns across 706 locales, 100% mapped
- `data/cldr/cldr_time_patterns.tsv` — 2824 time patterns across 706 locales
- `data/cldr/cldr_month_names.tsv` — Month names for 706 locales (wide + abbreviated)
- `data/cldr/cldr_weekday_names.tsv` — Weekday names for 706 locales (wide + abbreviated)
- `data/cldr/cldr_mapping_report.txt` — Coverage analysis

## Key findings

- **All 2823 CLDR date patterns map to existing FineType types** — no new taxonomy types needed
- Date format distribution: eu_slash (468), abbreviated_month (628), long_full_month (706), weekday_full_month (700), iso (175), eu_dot (113), us_slash (33)
- Time format split: 24h dominates (1422 hms_24h + 475 hm_24h) vs 12h (696 hms_12h + 231 hm_12h)
- 19 CJK locales excluded — CJK date patterns (y年M月d日) require new taxonomy types
- Month/weekday names available for 706 locales vs current 12 in locale_data.rs — Phase 2 will expand from 12 to 20+ locales

## Tests

- `cargo test` — 214 passed, 0 failed
- `cargo run -- check` — 171/171 generators pass, 100% alignment"
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
