# Memo: 0.7.0 enum accuracy reframe (deferred half of choice 0102)

**Date:** 2026-06-17
**Source decision:** `choice 0102` (categorical is an enum property, not a competing leaf)
**Patch already shipped:** `spec 2026-06-17-enum-domain-emission` (open — additive `x-finetype-enum` output; no model/gold/eval change)

## What this is

The **second, deferred increment** of choice 0102 — the part that touches the
model, gold, and eval (hence 0.7.0, not a patch). The patch shipped the
analyst-facing half (open enum-domain emission). This is the remainder:

- Drop `representation.discrete.categorical` as a terminal classification target
  (it is already untrained — emitted only by Sharpen rules; this retires those).
- **Gold migration**: re-label `categorical` gold columns to their semantic type
  (or a string/text residual) + an enum flag.
- **Eval reframe**: score the semantic type; enum-ness becomes a separate
  precision/recall dimension, not a competing label.
- Decide the **residual label** (what carries "bounded, no semantic type").
- **Closed-enum dictionaries**: native JSON-Schema `enum` keyword for genuinely
  closed domains (ISO codes, currencies) + out-of-domain validation. (This is also
  the "native `enum` vs x-finetype-enum" path — closed only, with a completeness
  guarantee, or it re-creates the enum_overfit bug, card 0014.)

## DO NOT mistake this for an accuracy play

Measured, not assumed: the reframe moves the headline **~−3, within noise**
(cardinality re-adjudication dry-run, 2026-06-17). The categorical boundary error
is **real, not a labelling artifact** (memory `cardinality-boundary-error-is-real`),
and `categorical` is a residual doing useful recall work — removing it loses that
recall, roughly offsetting any gain from no longer penalising the boundary. So:

- **Value here = ontology cleanliness + the honest model**, NOT headline points.
- Sell it as a correctness/architecture bet. If it gets framed as an accuracy
  lever, that's wrong and the session will over-claim (see the 2026-06-17
  separator-bug retraction for the over-read failure mode).

## Precondition before specing this

Settle the **representative-instrument baseline first** (the next-session lever):
gold's ~0.80 is a contested-curated hard slice (instrument audit: v19 = 68% on
contested ground). If the model is materially higher on representative data, the
gold migration's shape — and whether 98% is even reachable on this architecture —
changes. Don't spec the reframe until we know which instrument 98% is measured on.

## When picked up

Take 0102's deferred scope into `/design` → `/spec` against card 0002
(semantic-type-detection) and/or card 0019 (gold corpus). Honest goal statement:
"correct the model's ontology; zero expected headline movement."

## Related substrate

- `choice 0102` (the decision + patch decomposition)
- memories `cardinality-boundary-error-is-real`, `enum-domain-emission-shipped`,
  `categorical-is-a-residual-category`, `full-column-stat-sharpen-is-redundant`
- `spec 2026-06-17-cardinality-boundary-readjudication` (the −3 measurement)
