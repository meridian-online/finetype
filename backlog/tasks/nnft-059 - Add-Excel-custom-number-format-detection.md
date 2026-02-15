---
id: NNFT-059
title: Add Excel custom number format detection
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-14 10:08'
updated_date: '2026-02-15 08:54'
labels:
  - taxonomy
  - generator
  - feature
dependencies: []
references:
  - >-
    https://learn.microsoft.com/en-us/dotnet/standard/base-types/formatting-types
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Detect Excel/spreadsheet custom number format strings that commonly appear in exported data. These format codes tell spreadsheet applications how to display numbers and are found in metadata, headers, or as literal strings in data exports.

Common Excel format patterns:
- Number: `#,##0.00`, `0.00`, `#,##0`
- Currency: `$#,##0.00`, `€#,##0.00`, `[$$-409]#,##0.00`
- Percentage: `0.00%`, `0%`
- Date: `mm/dd/yyyy`, `d-mmm-yy`, `dddd, mmmm dd, yyyy`
- Time: `h:mm:ss AM/PM`, `[h]:mm:ss`
- Scientific: `0.00E+00`
- Custom: `#,##0.00;[Red]-#,##0.00;0.00;"text"` (positive;negative;zero;text sections)

This is especially relevant for GitTables data (sourced from spreadsheets) where format strings may appear as column metadata.

Reference: https://learn.microsoft.com/en-us/dotnet/standard/base-types/formatting-types
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Excel number format string type added to taxonomy
- [x] #2 Generator produces common Excel format patterns (number, currency, date, percentage)
- [x] #3 Detection distinguishes format strings from regular text
- [x] #4 DuckDB transformation contract documented (likely VARCHAR passthrough)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added representation.file.excel_format to definitions_representation.yaml with validation pattern covering all Excel format tokens (#, 0, commas, semicolons, brackets, currency symbols $€£¥, percent, date/time codes, conditional operators, fractions with ?).

Generator in generator.rs produces 8 format categories:
1. Number formats (#,##0, #,##0.00, variable decimal places)
2. Currency formats ($, €, £, ¥ with grouping)
3. Percentage formats (0%, 0.00%)
4. Date formats (10 variants: mm/dd/yyyy, yyyy-mm-dd, d-mmm-yy, etc.)
5. Time formats (7 variants: h:mm:ss AM/PM, hh:mm, etc.)
6. Scientific notation (0.00E+00 with variable decimals)
7. Fraction formats (# ?/?, # ??/??, # ?/2, # ?/4, # ?/8)
8. Conditional/multi-section formats (#,##0.00;(#,##0.00), [Red], [>100] conditions)

Fixed validation pattern twice: first added ? for fraction formats, then added ¥ and <> for conditional formats.

168 types, 8400/8400 samples pass, 169 tests pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Excel custom number format string detection (representation.file.excel_format).

Changes:
- New type definition in definitions_representation.yaml with validation pattern covering all Excel format tokens
- Generator produces 8 format categories: number, currency, percentage, date, time, scientific, fraction, conditional/multi-section
- DuckDB transform: VARCHAR passthrough (format strings preserved as-is)
- Validation pattern: ^[#0.,;\[\]$€£¥%EeAaPpMm/dDyYhHsS ?:"\-+()<>\w]*$ with minLength 2, maxLength 100

Taxonomy: 168 types, 8400/8400 samples pass, 169 tests pass.
<!-- SECTION:FINAL_SUMMARY:END -->
