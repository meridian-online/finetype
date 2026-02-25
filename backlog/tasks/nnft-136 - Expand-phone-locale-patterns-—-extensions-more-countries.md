---
id: NNFT-136
title: 'Expand phone locale patterns — extensions, more countries'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-25 11:39'
updated_date: '2026-02-25 12:21'
labels:
  - accuracy
  - validation
dependencies:
  - NNFT-132
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
NNFT-132 investigation found that only 23/254 SOTAB cardinality-demoted phone columns pass existing locale patterns. The validation precision fix (gating Signal 3 with locale_confirmed) rescues those 23, but the remaining 231 need expanded locale coverage.

Specific gaps identified in SOTAB data:
- Phone extensions: '(847) 945-4636 Work', '615.966.6280 (ext. 6280)', '(239) 939-1400 Ext. 103', '201-880-7213 x117'
- German (0) trunk prefix: '+49 (0)7721 - 90 88 70'
- South African numbers: '041 364 2260', '+27 215117818' (no ZA locale)
- German en-dash separators: '089 – 218 965 757'

Additionally, 14 SOTAB columns were demoted by Signal 1 (validation failure) due to these format gaps.

Fix approach: Add optional extension suffix to all locale patterns, add ZA locale, review DE pattern for (0) trunk prefix.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 EN_US/EN_CA patterns accept optional extension suffix (ext/Ext/x/Work/Cell)
- [x] #2 DE pattern handles (0) trunk prefix notation
- [x] #3 ZA locale pattern added for South African phone numbers
- [x] #4 All tests pass — cargo test + taxonomy check
- [x] #5 SOTAB re-eval shows measurable improvement in telephone column accuracy
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add extension suffix support to EN_US and EN_CA patterns
   - Optional trailing: (ext./Ext./x/# + 1-5 digits) or (Work/Cell/Fax/Home/Office)
   - Pattern fragment: (\s*(ext\.?|Ext\.?|x|#)\s*\d{1,5})?(\s*(Work|Cell|Fax|Home|Office))?$

2. Fix DE pattern for (0) trunk prefix and / separator
   - Add (\(0\))? after +49 country code group
   - Add / to allowed separators alongside space, hyphen, dot
   - Also handle en-dash (–, U+2013) as separator

3. Update EN_GB pattern for (0) trunk prefix
   - +44(0) is a common UK convention: +44(0)2476527600
   - Add (\(0\))? after +44 country code group

4. Update FR pattern for (0) trunk prefix
   - +33 (0)1 40 70 11 80 is common French convention
   - Add (\(0\))? after +33 country code group

5. Add ZA (South African) locale pattern
   - Format: +27 optional, then 0? prefix, 9-10 digits with flexible grouping
   - Examples: +27 215117818, 041 364 2260, 076 028 6546

6. Add extension suffix to universal validation pattern
   - The universal pattern (minLength/maxLength) needs maxLength increased for extensions

7. Run cargo test + taxonomy check
8. Rebuild release binary and run SOTAB eval
9. Compare telephone accuracy before/after
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
SOTAB eval results (NNFT-132 + NNFT-136 combined):
- Format-detectable: 39.5% → 42.5% label (+3.0pp), 59.5% → 62.6% domain (+3.1pp)
- Telephone: 236/486 correct (48.6% label), cardinality demotions 254 → 24, validation demotions 14 → 7
- Profile eval: 70/74 unchanged
- All 174 model tests pass, taxonomy check clean
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded phone number locale patterns to address SOTAB validation gaps identified in NNFT-132 investigation.

## Changes

**labels/definitions_identity.yaml** — Comprehensive phone pattern expansion:
- Extension suffix support added to all 15 locale patterns: optional trailing `(ext./Ext./x/# + digits)` or `(Work/Cell/Fax/Home/Office)` labels
- Separator character class expanded from `[\s\-.]` to `[\s\-./\u2013]` (added slash for German formats, en-dash U+2013)
- Trunk prefix `(\(0\))?` added to EN_GB, EN_AU, DE, FR, ZA after country code group
- New ZA (South African) locale: `+27` optional, `0`-prefix optional, 9-10 digits with flexible grouping
- Universal validation: maxLength 20→30, pattern updated with slash/en-dash/extension support
- All locale maxLength values standardized to 35

## Impact

SOTAB format-detectable accuracy (combined NNFT-132 + NNFT-136):
- Label: 39.5% → 42.5% (+3.0pp)
- Domain: 59.5% → 62.6% (+3.1pp)
- Telephone cardinality demotions: 254 → 24 (230 columns rescued)
- Telephone validation demotions: 14 → 7
- Profile eval: 70/74 unchanged (no regressions)

## Tests
- cargo test: 174 model tests pass (272 total)
- cargo run -- check: all 169 definitions pass taxonomy alignment
- Profile eval: 70/74 unchanged
- SOTAB eval: measurable improvement confirmed
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
