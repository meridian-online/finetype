---
status: accepted
date-created: 2026-05-04
date-modified: 2026-05-04
---
# 0082. Inference fallback policy — text.string vs unknown

## Context and Problem Statement

The triangulator (MADR 0079) needs a defined behaviour for two
cases where it cannot return a confident specific taxonomy type:
(1) the column has observed values but no candidate type's score
clears the fallback threshold ("read it but no canonical type
fits"); (2) the column has no observed values to score against
("cannot read this column"). Both cases need an output, but
treating them identically loses signal: case (1) has a column
shape the module saw and rejected, case (2) has no information
at all. The output choice affects what E01a's pair-distinctness
threshold counts as a meaningful disagreement.

## Considered Options

- **Option A — Strict unknown when confidence < threshold.**
  Both cases emit `inferred_correct_type = "unknown"`,
  confidence = 0.0, mechanism = `fallthrough`. Simple. Concedes
  the long tail; high non-unknown rate is harder to hit. Loses
  the case-(1) vs case-(2) distinction.
- **Option B — Generic fallback with low confidence.** Case (1)
  emits `inferred_correct_type = "representation.text.string"`,
  confidence = 0.3, mechanism = `unknown_no_fit`. Case (2) emits
  `inferred_correct_type = "unknown"`, confidence = 0.0,
  mechanism = `fallthrough`. Two distinct outputs preserve the
  signal; case-(1) emissions still count as "non-unknown" for
  ac-02's floor measurement, keeping E01a viable for novel
  shapes.
- **Option C — Cascade through generic super-types.** Case (1)
  tries `representation.text.string` → `representation.discrete.identifier`
  → `representation.discrete.categorical` → ... → `unknown`,
  emitting whichever super-type's validator scores best. More
  predictions classified, but signal-quality varies by tier and
  the super-type validators are not guaranteed to be tighter
  than `text.string` (which passes everything).

## Decision Outcome

Chosen option: **Option B — generic fallback with low
confidence**, because it preserves the case-(1) vs case-(2)
distinction in the mechanism column (the analyst can audit
"what shapes did we see but couldn't type?" via `unknown_no_fit`
separately from "what columns had no data?" via `fallthrough`)
without inflating the architecture (Option C's super-type
cascade requires per-super-type validator quality assertions
this bead doesn't ship).

The placement is structural:
- Case (2) — empty samples — is a **precondition guard** before
  the rule cascade runs. The cascade cannot evaluate scores
  without samples; checking emptiness first is the only
  architecturally consistent placement.
- Case (1) — non-empty samples but `max_score <
  fallback_threshold` (= 0.4) — is **Rule 1** of the cascade,
  fired before any structural rule (validator_widening,
  enum_overfit, etc.) can fire. Rationale: if no candidate
  scored well enough to be confident in, structural rules
  about *which* candidate match what failure mode are moot.

The confidence values are deliberate:
- `confidence = 0.0` for `fallthrough` signals "no inference
  attempted" — distinct from any non-zero score and never
  counted as non-unknown for ac-02 purposes.
- `confidence = 0.3` for `unknown_no_fit` signals "inference
  attempted, low quality" — counted as non-unknown by definition
  (`inferred_correct_type != "unknown"`), but below ac-02's 0.7
  threshold so it doesn't inflate the floor measurement.
- `inferred_correct_type = "representation.text.string"` is the
  canonical generic fallback because (a) `text.string`'s
  validator is explicitly the most-permissive in the taxonomy;
  (b) emitting it is a positive signal "I see characters" rather
  than a passive "I see nothing"; (c) downstream consumers
  reading the failure_log can distinguish text.string-fallback
  from un-typed via the mechanism column.

### Consequences

- Good, because E01a's pair-distinctness threshold can fire on
  novel shapes that don't match any specific taxonomy entry —
  case (1) emissions accumulate as `(predicted, text.string)`
  pairs with mechanism `unknown_no_fit`, which is a real signal
  that the corpus contains a shape worth a new taxonomy entry.
- Good, because the case-(1) vs case-(2) distinction lets the
  contract's E04 escalation triage:
  many `unknown_no_fit` → "we should add a new type to the
  taxonomy"; many `fallthrough` → "we should investigate why so
  many columns have no data" (likely a parser bug or data
  quality issue).
- Good, because confidence 0.3 sits below ac-02's 0.7 threshold,
  so case-(1) emissions are decisive (non-unknown) but don't
  inflate the floor — the floor measures *high-confidence*
  decisiveness, not catch-all decisiveness.
- Bad, because `text.string` as a fallback type can be
  mis-interpreted by downstream consumers who don't read the
  mechanism column. A naive consumer of failure_log seeing
  `text.string` may assume "the model thought this was a typed
  column but it's actually plain text" rather than "the inference
  module couldn't type it." Mitigated: the mechanism column is
  the source of truth; consumers reading `inferred_correct_type`
  in isolation are misusing the schema.
- Neutral, because Option C's super-type cascade is preserved as
  a possible Phase-2 evolution. If `unknown_no_fit` becomes the
  most common inference outcome, a future card can promote the
  cascade through generic super-types as a way to extract more
  signal from the long tail.

References:
- `orbit/specs/2026-05-04-autonomous-type-inference/spec.yaml`
  (ac-09 cascade Rule 1 + precondition; ac-10 verification)
- `orbit/specs/2026-05-04-autonomous-type-inference/interview.md` (Q5)
- `orbit/decisions/0081-mechanism-vocabulary-aligned-with-madr-0075.md`
