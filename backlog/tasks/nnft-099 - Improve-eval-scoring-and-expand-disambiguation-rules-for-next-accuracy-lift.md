---
id: NNFT-099
title: Improve eval scoring and expand disambiguation rules for next accuracy lift
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-18 03:39'
updated_date: '2026-02-18 03:39'
labels:
  - accuracy
  - disambiguation
  - eval
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Profile eval at 76.1% (86/113) format-detectable label accuracy after NNFT-090. Analysis of remaining 27 errors reveals 4-7 fixable columns across three categories:

**Eval scoring refinements:**
- Time sub-types (hm_24h vs hms_24h) should be interchangeable — sports_events.start_time uses HH:MM format, model correctly predicts hm_24h but GT maps to hms_24h (+1)
- Geographic hierarchy (continent ≈ region ≈ state) should be interchangeable — countries.region/sub-region values ARE continents (Asia, Europe), model is correct (+2-3)

**Disambiguation rule fixes:**
- Gender rule: "Non-binary" not in gender value set → disambiguate_gender doesn't fire for people_directory.gender (+1)

**Header hint improvements:**
- identity.person.username should be added to is_generic type list — it's a catch-all for short unrecognized text. covid_timeseries.Country predicted as username (0.82 conf) but header hint can't override because username isn't flagged as generic (+1-2)

Target: ~80% format-detectable label accuracy (90+/113)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Format-detectable label accuracy improves beyond 76.1% (86/113)
- [ ] #2 Eval SQL treats time sub-types and geographic hierarchy as interchangeable where appropriate
- [ ] #3 Gender disambiguation rule handles Non-binary and other inclusive gender values
- [ ] #4 Header hint overrides work for username-predicted columns with matching headers
- [ ] #5 No regression on existing correct classifications
- [ ] #6 Unit tests for gender rule expansion
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Eval SQL: Add time sub-type interchangeability (hm_24h ≈ hms_24h ≈ hm_12h etc.)
2. Eval SQL: Add geographic hierarchy interchangeability (continent ≈ region ≈ state)
3. Gender rule: Expand GENDER_VALUES with Non-binary, Other, Prefer not to say, etc.
4. Header hint: Add username to is_generic type list
5. Unit tests for expanded gender rule
6. Build and run full test suite
7. Re-run profile eval to verify improvements and zero regressions
8. Commit and push
<!-- SECTION:PLAN:END -->
