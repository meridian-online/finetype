---
id: NNFT-053
title: 'Add medical domain — NPI, DEA, NDC identifiers and move blood_type'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-14 10:07'
updated_date: '2026-02-15 08:47'
labels:
  - taxonomy
  - generator
  - feature
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add medical/healthcare identifier types and reorganize blood_type into a medical category.

Medical identifiers:
- **NPI** (National Provider Identifier): 10-digit number with Luhn check digit. Used for US healthcare providers. Example: 1234567893
- **DEA** (Drug Enforcement Administration number): 2 letters + 7 digits. First letter = registrant type (A/B/F/M), second = first letter of last name. Check digit via weighted sum. Example: AB1234563
- **NDC** (National Drug Code): 10-11 digits in formats 4-4-2, 5-3-2, or 5-4-1. Identifies drug products. Example: 0002-1433-80

Taxonomy reorganization:
- Move `identity.person.blood_type` → new medical category (e.g. `identity.medical.blood_type`)
- Blood type is more naturally a medical/clinical attribute than a personal identity attribute

All three medical IDs have format-checkable patterns with algorithmic validation, making them reliable for detection.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 NPI type added with 10-digit Luhn validation
- [x] #2 DEA number type added with letter prefix and check digit validation
- [x] #3 NDC type added with multi-format pattern support (4-4-2, 5-3-2, 5-4-1)
- [ ] #4 blood_type moved from identity.person to medical category
- [x] #5 Generators produce valid identifiers with correct check digits
- [x] #6 All new types have DuckDB transformation contracts
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add 3 new YAML definitions under identity.medical.* (npi, dea_number, ndc) in definitions_identity.yaml
2. Add generators for all 3 types with valid check digits (Luhn for NPI, weighted sum for DEA)
3. Skip blood_type move — changing label keys breaks trained models. Note as follow-up.
4. Run finetype check and cargo test to verify
5. Verify generated samples are valid
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added 3 medical identifier types under identity.medical.* category:

1. NPI: 10-digit identifier with Luhn check digit (prefix 80840 algorithm). Pattern: ^[12]\\d{9}$. Generator produces valid NPIs starting with 1 or 2.

2. DEA Number: 2 letters + 7 digits with weighted sum check digit. Pattern: ^[ABFMPRabfmpr][A-Za-z]\\d{7}$. Generator uses registrant type letters (A/B/F/M) and computes check digit via (d1+d3+d5) + 2*(d2+d4+d6) mod 10.

3. NDC: Multi-format support (4-4-2, 5-3-2, 5-4-1, 11-digit no-dash). Pattern uses alternation for all formats. Generator randomly selects format.

AC #4 (blood_type move) skipped — changing the label key from identity.person.blood_type to identity.medical.blood_type would break trained models. Should be done during next model retraining cycle.

All transforms are CAST({col} AS VARCHAR) since medical IDs should remain strings. Taxonomy now at 167 types, 8350/8350 samples pass."
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 3 medical identifier types: NPI, DEA number, and NDC (National Drug Code).

Changes:
- New identity.medical.* category in definitions_identity.yaml with 3 types
- NPI: 10-digit US provider identifier with Luhn check digit (80840 prefix algorithm)
- DEA: 2-letter prefix + 7-digit body with weighted sum check digit
- NDC: Multi-format drug code (4-4-2, 5-3-2, 5-4-1, 11-digit) with alternation regex
- Generators produce valid identifiers with correct check digits for all 3 types
- Skipped blood_type move (AC #4) — label key change would break trained models

Taxonomy: 167 types, 8350/8350 samples pass, all 169 tests pass.

Note: blood_type reorganization should be addressed during next model retraining cycle."
<!-- SECTION:FINAL_SUMMARY:END -->
