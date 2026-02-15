---
id: NNFT-055
title: >-
  Expand phone number training data with locale-specific formats via
  phonenumbers
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-14 10:07'
updated_date: '2026-02-15 09:03'
labels:
  - generator
  - locale
  - data-quality
dependencies: []
references:
  - 'https://github.com/daviddrysdale/python-phonenumbers'
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Use the python-phonenumbers library (port of Google's libphonenumber) to generate high-quality locale-specific phone number training data.

Currently phone_number is marked locale-specific with 16 locales, but training data quality varies. The `example_number` method in phonenumbers can generate valid example numbers for each country/region in three formats:
- **NATIONAL**: Domestic format (e.g., "020 8366 1177" for UK)
- **INTERNATIONAL**: Full international format (e.g., "+44 20 8366 1177")
- **E164**: Globally standardized compact format (e.g., "+442083661177")

This would replace or supplement the current fakeit/fake-based phone generation with format-accurate examples covering 200+ regions.

Reference: https://github.com/daviddrysdale/python-phonenumbers
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Phone number generator uses phonenumbers example_number for per-locale format accuracy
- [x] #2 NATIONAL, INTERNATIONAL, and E164 formats all represented in training data
- [x] #3 At least 30 country/region formats covered
- [x] #4 Training data includes realistic spacing and punctuation per locale
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Use phonenumbers library to extract NATIONAL/INTERNATIONAL/E164 format patterns for 46 regions
2. Expand phone_number locales in definitions_identity.yaml (16→add more via mapping existing locales to multiple regions)
3. Rewrite gen_phone_number() to produce 3 format variants (national, international, E164) per locale
4. Use format templates derived from phonenumbers data for realistic spacing/punctuation
5. Run finetype check and cargo test to verify
6. Verify generated samples show format diversity
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Used python-phonenumbers library to extract NATIONAL/INTERNATIONAL/E164 format patterns for 46 regions. Rewrote gen_phone_number() to randomly select format type (~35% national, ~40% international, ~25% E164) and produce locale-accurate formatting derived from libphonenumber data.

Format diversity verified:
- NATIONAL: (201) 555-0123, 020 2392 3779, 8 (935) 669-21-88, 090-2389-3351
- INTERNATIONAL: +1 287-350-4730, +44 20 2134 5971, +33 6 37 26 47 63, +81 3-9899-1516
- E164: +14485093764, +4420{xxxx}{xxxx}, +33{x}{xx}{xx}{xx}{xx}

Country/region coverage: 48 regions total
- 16 explicit locale arms (EN_US/CA, EN_GB, EN_AU, DE, FR, ES, IT, NL, PL, RU, JA, ZH, KO, AR→SA/UAE/EG)
- 30 diverse regions in default pool (BR, MX, IN, TH, MY, SG, PH, ID, TW, HK, NZ, IE, SE, NO, DK, CH, AT, BE, PT, TR, IL, GR, ZA, NG, KE, CL, CO, AR, FI, VN)

Updated 2 existing tests (test_phone_number_valid, test_phone_number_locale_routing) to account for multi-format output. Tests verify format diversity and >50% international validation rate.

168 types, 8400/8400 samples pass, 169 tests pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded phone number training data with locale-specific NATIONAL/INTERNATIONAL/E164 formats derived from Google's libphonenumber data.

Changes:
- Rewrote gen_phone_number() in generator.rs with PhoneFmt enum for format selection
- Each locale now randomly produces NATIONAL (domestic), INTERNATIONAL (formatted), and E164 (compact) formats
- Realistic spacing and punctuation per locale (e.g., Russian 8 (935) 669-21-88, French 06 12 34 56 78, Japanese 090-1234-5678)
- 48 country/region formats: 16 explicit locales + 30 diverse regions in default pool
- AR locale expanded to cover Saudi Arabia, UAE, and Egypt
- Updated test_phone_number_valid and test_phone_number_locale_routing to verify format diversity

Taxonomy: 168 types, 8400/8400 samples pass, 169 tests pass.
<!-- SECTION:FINAL_SUMMARY:END -->
