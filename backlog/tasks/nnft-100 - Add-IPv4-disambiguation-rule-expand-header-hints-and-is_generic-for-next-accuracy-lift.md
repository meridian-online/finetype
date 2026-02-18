---
id: NNFT-100
title: >-
  Add IPv4 disambiguation rule, expand header hints and is_generic for next
  accuracy lift
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-18 05:17'
labels:
  - accuracy
  - disambiguation
  - eval
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Profile eval at 81.4% (92/113) format-detectable label accuracy after NNFT-099. Analysis of remaining 21 errors reveals several fixable patterns:

**Value-level disambiguation:**
- IPv4 regex rule: destination_ip has real IPs predicted as version (0.52 conf)

**Header hint expansion:**
- IP-related: "source ip", "destination ip", etc.
- UTC offset: "utc offset", "gmt offset"
- Weight/height: h.contains("weight"/"height")
- Financial codes: CVV, SWIFT, ISSN, EAN
- OS: "os", "operating system"
- Subcountry: subcountry/subregion → state

**is_generic expansion:**
- Add representation.numeric.increment — unlocks age hint override

**Eval SQL:**
- Timestamp sub-type interchangeability (iso_8601 ≈ iso_8601_microseconds)

Target: ~85% format-detectable label accuracy (96+/113)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Format-detectable label accuracy improves beyond 81.4% (92/113)
- [ ] #2 IPv4 regex disambiguation rule correctly identifies IP addresses regardless of model prediction
- [ ] #3 Header hints added for IP, UTC offset, weight, height, CVV, SWIFT, ISSN, EAN, OS, subcountry patterns
- [ ] #4 representation.numeric.increment added to is_generic type list
- [ ] #5 No regression on existing correct classifications
- [ ] #6 Unit tests for IPv4 disambiguation rule
<!-- AC:END -->
