---
status: accepted
date-created: 2026-03-26
date-modified: 2026-03-26
---
# 0044. Widen CalVer Generator for Distinctiveness from SemVer

## Context and Problem Statement

The calver generator produced only `YYYY.MM` and `YYYY.MM.DD` formats. While the 4-digit
year prefix distinguishes calver from semver (which starts with small integers), the limited
format variety meant the model had fewer distinguishing signals. The collision audit (C-07)
flagged calver as a subset of version patterns.

## Considered Options

- **Option A:** Keep narrow calver generator, rely on header disambiguation
- **Option B:** Widen calver to include more real-world patterns (micro-versions, non-padded months)

## Decision Outcome

Chosen option: "Option B", because real-world calver usage includes Ubuntu-style
`YYYY.MM.DD.micro` and pip-style `YYYY.M` formats. Wider generator coverage gives the
model more distinctive training signal while remaining within the calver specification.

### Consequences

- Good, because generator now produces 4 format variants (YYYY.MM, YYYY.MM.DD, YYYY.MM.DD.micro, YYYY.M)
- Good, because validation pattern updated to accept 1-2 digit months
- Good, because year range widened (2018-2026) for more realistic training data
- Neutral, because semver already uses 3-part M.N.P with small integers — structural distinction remains clear
