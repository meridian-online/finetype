---
id: NNFT-060
title: Use CLDR release data as upstream source for locale-specific generators
status: To Do
assignee: []
created_date: '2026-02-14 10:08'
updated_date: '2026-03-04 01:55'
labels:
  - infrastructure
  - locale
  - data-quality
milestone: m-6
dependencies:
  - NNFT-045
references:
  - 'https://cldr.unicode.org/'
documentation:
  - 'https://cldr.unicode.org/'
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Integrate Unicode CLDR (Common Locale Data Repository) data as the authoritative source for all locale-specific training data generation.

CLDR v48.1 (Jan 2026) provides:
- **Date/time patterns** per locale (short/medium/long/full)
- **Number formatting** (decimal separator, grouping, percent, currency)
- **Currency info** (symbols, names, placement per locale)
- **Month/day names** (abbreviated, wide, narrow per locale)
- **Measurement units** and their locale-specific formatting
- **Language/country/timezone names** per locale

Rather than hardcoding locale patterns in individual generators, this task establishes CLDR XML/JSON data as the single source of truth. Generators for dates, numbers, currencies, addresses, and other locale-dependent types would read from CLDR data at training time.

This is the foundational data layer that tasks like locale phone numbers, locale dates, and locale addresses would build on.

Reference: https://cldr.unicode.org/
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CLDR data (XML or JSON) downloaded and accessible for training data generation
- [ ] #2 Parser extracts date/time patterns, number formats, and month/day names per locale
- [ ] #3 At least 10 locales fully parsed and validated
- [ ] #4 Generators can import CLDR data instead of hardcoded patterns
<!-- AC:END -->
