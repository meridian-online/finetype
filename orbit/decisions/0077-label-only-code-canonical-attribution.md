---
status: accepted
date-created: 2026-04-29
date-modified: 2026-04-29
---
# 0077. Label-only code-vs-canonical attribution (defer value-shape signals)

## Context and Problem Statement

The iter-3 cascade attributes `code_vs_canonical` failures using two
trigger paths:

- **path-a-pattern**: predicted == expected, SEMANTIC_TYPE pattern
  reject, AND column name in seam table.
- **path-b-codetype**: predicted ≠ expected, AND exactly one side
  (predicted XOR expected) is in `CODE_TYPED_LABELS`.

Both signals are **label-only** — the cascade sees the predicted
label, the expected label, the column name, and the reject row's
metadata (error_type, constraint_failed, constraint_value). It does
NOT see the actual cell values that triggered the rejects.

This works for the iter-2 anchors:
- FIFA Value (`€110.5M`) is `path-b-codetype` because the model
  predicts `finance.currency.amount` (in allowlist) against an
  expected `representation.text.plain_text` (not in allowlist) —
  cross-prefix XOR fires correctly without value inspection.
- OECD REF_AREA carrying ISO-3 codes against `geography.location.country_code`
  is `path-a-pattern` if the column name matched the seam table.

But the iter-2 corpus surfaced two cases where label-only signals
are insufficient:

- **GICS Sector** — values are full canonical names ("Information
  Technology", "Health Care") and the model classifies them as
  `representation.text.plain_text`, which is the correct broad type.
  There is no FineType label for "GICS sector taxonomy" so neither
  side is in the allowlist, and the prediction-vs-expected match is
  benign. Result: the column passes validation under iter-3. The
  iter-2 anchor was framed as a `code_vs_canonical` failure, but the
  cascade has no way to attribute it that way without either
  widening the taxonomy (a model+data card) or inspecting the values
  themselves (a harness card).
- **OECD REF_AREA model-misprediction collision** — the model
  predicts `technology.internet.http_method` (in `CODE_TYPED_LABELS`)
  against an expected `geography.location.country_code` (also in
  `CODE_TYPED_LABELS`). XOR=false so path-b-codetype does not fire;
  cascade routes to misclassification. The label-only signal cannot
  distinguish "the model picked a code label that's wrong because
  the values are 3-letter country codes, not HTTP verbs" from a
  generic cross-domain misclassification.

Iter-3 needs a decision: extend the cascade to inspect cell values, or
ship label-only signals and document the gap.

## Considered Options

- **Option A — Plumb cell values into the cascade now.** Extend
  `RejectRow` (or add a parallel `ColumnSample` struct) to carry a
  sample of cell values per failing column. Add a value-shape
  predicate (length distribution, alphabet, regex hit rate) that
  fires when the model's predicted code-typed label is contradicted
  by the value shape. Catches REF_AREA's model-misprediction
  collision and gives a path to GICS Sector attribution.
- **Option B — Ship label-only; document gaps as
  `pending_escalation: true` fixture rows; file follow-up cards.**
  Iter-3 ships with the existing 7-rule cascade. REF_AREA and GICS
  Sector are recorded in the fixture with their current (label-only)
  attribution AND `pending_escalation: true`; the rationale field
  names the upstream fix path (model improvement for REF_AREA;
  taxonomy widening + value-shape signals for GICS Sector).
- **Option C — Add a value-shape predicate ONLY to break path-b
  ties (allowlist XOR=false).** Narrower than A: the cascade still
  uses label-only signals everywhere except the misclassification
  catch-all, where a value-shape check could promote the row to
  code_vs_canonical when the predicted code label's shape doesn't
  match the values. Solves REF_AREA but not GICS Sector.

## Decision Outcome

Chosen option: **Option B — ship label-only attribution; document
gaps via `pending_escalation`**, because it preserves the iter-3
spec's scope (3 hard anchors + 2 known-gap rows) without expanding
the cascade's input surface mid-iteration. The cascade's input
contract — `(predicted_label, expected_label, column_name, RejectRow
metadata)` — stays minimal and is easy to test with synthetic
fixtures (the `vci3_attribute_*` positive/negative tests in
`mod attribute_tests`).

The label-only constraint is honest about what the cascade currently
knows. Promoting it to a value-aware classifier is a meaningful
expansion that deserves its own card with explicit acceptance criteria
around value-sample size, performance, and a regression test for the
two iter-2 escalation cases (REF_AREA and GICS Sector). Doing it
inline as a one-off "while we're here" change risks under-thinking
the value-shape predicate.

The two pending rows in iter-3's fixture document the escalation
path:

- `oecd_employment.REF_AREA` — `expected_mechanism: code_vs_canonical`,
  `pending_escalation: true`, rationale names model improvement
  (lift `http_method` out of REF_AREA's confusion set in the next
  retrain) AND value-shape signals as the two viable escalation
  paths.
- `sp500_constituents.GICS Sector` — `expected_mechanism: code_vs_canonical`,
  `pending_escalation: true`, rationale names taxonomy widening
  (add a GICS sector label) OR value-shape signals as the
  escalation path.

### Consequences

- Good, because the cascade's input contract stays minimal —
  label + reject metadata. Adding new rules requires no changes
  to upstream profile/validate code.
- Good, because the iter-2 escalation truth (REF_AREA and GICS
  Sector are real `code_vs_canonical` cases) is preserved in the
  fixture without forcing the cascade to make a claim it can't
  defend with the inputs it has.
- Good, because the follow-up cards inherit a clear spec: "extend
  the cascade to use value-shape signals; the regression test is
  that the two `pending_escalation: true` rows in the fixture flip
  to `pending_escalation: false`."
- Bad, because the iter-3 report's per-mechanism breakdown
  under-reports `code_vs_canonical` by 2 (REF_AREA and GICS Sector
  attribute elsewhere). Mitigated: the fixture documents the
  ground truth and the report's row-level table shows the actual
  attribution alongside it.
- Neutral, because the cascade's `path-b-codetype` rule already
  has the seam-table override (Rule 5 wins over Rule 4 for code/
  canonical column names) — the label-only signal is sufficient
  for the load-bearing iter-2 anchors (FIFA Value, FIFA Wage)
  even without value-shape extension.
