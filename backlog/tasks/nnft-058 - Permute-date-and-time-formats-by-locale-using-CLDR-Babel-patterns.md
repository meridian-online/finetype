---
id: NNFT-058
title: Permute date and time formats by locale using CLDR/Babel patterns
status: To Do
assignee: []
created_date: '2026-02-14 10:08'
updated_date: '2026-03-04 01:55'
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
- [ ] #1 CLDR date/time patterns extracted for at least 10 locales
- [ ] #2 Training data generated for short/medium/long/full format levels per locale
- [ ] #3 Each generated sample maps to a valid DuckDB strptime format string
- [ ] #4 Pattern permutation covers locale-specific ordering (DMY, MDY, YMD)
<!-- AC:END -->
