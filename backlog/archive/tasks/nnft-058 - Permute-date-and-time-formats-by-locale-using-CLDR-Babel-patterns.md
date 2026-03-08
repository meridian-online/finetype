---
id: NNFT-058
title: Permute date and time formats by locale using CLDR/Babel patterns
status: Done
assignee: []
created_date: '2026-02-14 10:08'
updated_date: '2026-03-08 06:04'
labels:
  - generator
  - locale
  - datetime
milestone: m-6
dependencies:
  - NNFT-045
references:
  - 'https://babel.pocoo.org/en/latest/dates.html'
  - 'https://cldr.unicode.org/'
documentation:
  - 'https://cldr.unicode.org/'
  - 'https://babel.pocoo.org/en/latest/dates.html'
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generate locale-specific date and time training data using CLDR pattern data (as exposed by libraries like Babel).

CLDR defines four standard date format levels per locale:
- **short**: "4/1/07" (en-US) vs "01/04/07" (fr-FR)
- **medium**: "Apr 1, 2007" vs "1 avr. 2007"
- **long**: "April 1, 2007" vs "1 avril 2007"
- **full**: "Sunday, April 1, 2007" vs "dimanche 1 avril 2007"

Each uses LDML pattern syntax (y=year, M=month, d=day, E=weekday, etc.) which varies by locale.

This task would:
1. Extract date/time patterns from CLDR data for target locales
2. Generate training samples using each pattern variation
3. Map each pattern to the correct strptime format for DuckDB transformation

Critical dependency: NNFT-045 must decide on locale strategy first, since DuckDB strptime only supports English month/day names.

Reference: https://babel.pocoo.org/en/latest/dates.html
Reference: https://cldr.unicode.org/
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CLDR date/time patterns extracted for at least 10 locales
- [x] #2 Training data generated for short/medium/long/full format levels per locale
- [x] #3 Each generated sample maps to a valid DuckDB strptime format string
- [x] #4 Pattern permutation covers locale-specific ordering (DMY, MDY, YMD)
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Closed as superseded. The substance of this task was delivered by subsequent work:

- AC #1 (CLDR extraction): NNFT-157 extracted 2823 date + 2824 time patterns for 700+ locales, all mapped to FineType types.
- AC #2 (Training data generation): NNFT-158 enriched generators with CLDR locale data for 6 locale-specific datetime types across 30+ locales. CharCNN-v11 through v14 all trained on this data.
- AC #3 (strptime format strings): Partially addressed — format_string exists per type but isn't locale-varied. Moot for DuckDB (strptime is English-only). Low-priority follow-up task created.
- AC #4 (DMY/MDY/YMD ordering): NNFT-158 implemented date_format_pattern() with locale-appropriate ordering.

This task predated the taxonomy expansion from ~20 to 84 datetime types and the CLDR data infrastructure buildout. The original Babel/Python approach was also superseded by the zero-Python architecture.
<!-- SECTION:FINAL_SUMMARY:END -->
