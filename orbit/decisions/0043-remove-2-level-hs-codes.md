---
status: accepted
date-created: 2026-03-26
date-modified: 2026-03-26
---
# 0043. Remove 2-Level HS Codes from Generator and Validation

## Context and Problem Statement

The hs_code generator produced 2-level codes (e.g., `8471.30`) at a 10% rate. These are
indistinguishable from plain decimal numbers without header context. The collision audit
(C-01) identified this as a HIGH-severity collision with `representation.number.decimal`.

Analysts encountering a column of values like `8471.30` without an "HS Code" header would
reasonably expect them to be classified as decimals. The 3+ level formats
(e.g., `0901.11.00.10`) are structurally distinctive and worth training on.

## Considered Options

- **Option A:** Keep 2-level, rely on header disambiguation
- **Option B:** Remove 2-level from generator and tighten validation pattern

## Decision Outcome

Chosen option: "Option B", because 2-level HS codes are genuinely ambiguous with decimals
and the structural signal comes from the dotted multi-level format. Values like `8471.30`
should be classified as decimal when presented without header context.

### Consequences

- Good, because training data no longer contains decimal-lookalike HS codes
- Good, because the model learns structurally distinctive patterns only
- Good, because validation pattern (`^\d{4}\.\d{2}(\.\d{2}){1,2}$`) now matches generator output
- Bad, because real-world HS codes at chapter.heading level (4+2 digits) will classify as decimal — this is acceptable per the analyst perspective
