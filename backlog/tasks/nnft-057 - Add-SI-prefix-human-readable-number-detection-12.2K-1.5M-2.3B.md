---
id: NNFT-057
title: 'Add SI-prefix human-readable number detection (12.2K, 1.5M, 2.3B)'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-14 10:08'
updated_date: '2026-02-15 08:43'
labels:
  - taxonomy
  - generator
  - feature
dependencies: []
references:
  - 'https://github.com/debrouwere/python-ballpark'
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add a new type for human-readable numbers with SI/business notation suffixes. These are extremely common in dashboards, reports, and spreadsheet data.

Formats to detect:
- K/k (thousands): "12.2K", "500k"
- M/m (millions): "1.5M", "3.2m"  
- B/b (billions): "2.3B"
- T/t (trillions): "1.1T"
- Optional currency prefix: "$1.5M", "€2.3B"
- Optional sign: "-500K", "+1.2M"

The DuckDB transformation contract would parse the suffix and multiply: "12.2K" → 12200.

Inspired by python-ballpark library which calls this "business notation."

Reference: https://github.com/debrouwere/python-ballpark
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 New type added for SI-prefix numbers (e.g., representation.numeric.si_number)
- [x] #2 Generator produces K/M/B/T suffixed values with varied precision
- [x] #3 DuckDB transformation contract parses suffix and converts to numeric value
- [x] #4 Detection distinguishes SI numbers from plain text (e.g., 'OK' is not a number)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add YAML definition to definitions_representation.yaml with validation pattern and transform
2. Add generator function in generator.rs for si_number type
3. Run finetype check to verify alignment
4. Run cargo test to verify no regressions
5. Verify generated samples look correct
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added representation.numeric.si_number type with:
- YAML definition with validation pattern ^[\\$€£+-]?\\d+\\.?\\d*[KkMmBbTt]$ 
- DuckDB CASE transform that strips prefix and multiplies by K/M/B/T factor
- Generator produces K/M/B/T suffixes with varied precision (0-2 decimals), optional currency prefix ($, €, £), and optional sign (+/-)
- Pattern requires trailing suffix letter to distinguish from plain numbers (AC #4)
- Taxonomy now at 164 types, 8200/8200 samples pass, all 169 tests pass"
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added SI-prefix number detection type (representation.numeric.si_number).

Changes:
- New YAML definition in definitions_representation.yaml with validation pattern, DuckDB CASE transform (strips prefix, multiplies by K/M/B/T factor), and 5 sample values
- Generator in generator.rs produces varied samples: K/M/B/T suffixes, case variation, 0-2 decimal precision, optional currency prefix ($, €, £), optional sign (+/-)
- Pattern requires trailing suffix letter — distinguishes '12.2K' from plain numbers like 'OK'
- Taxonomy: 164 types, 8200/8200 samples passing, all 169 tests pass"
<!-- SECTION:FINAL_SUMMARY:END -->
