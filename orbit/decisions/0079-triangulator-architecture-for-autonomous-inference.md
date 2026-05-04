---
status: accepted
date-created: 2026-05-04
date-modified: 2026-05-04
---
# 0079. Triangulator architecture for autonomous type inference

## Context and Problem Statement

The 2026-05-10 GitTables 90% round-trip contract's E01a escalation
needs a cycle-worker module that, given `(predicted_type, samples,
column_name)`, infers what the *correct* taxonomy type should be when
the model has misclassified a column. The naive approach is
"validator retrieval": run all 240 validators over the column's
samples and pick the type with highest pass-rate. The interview-Q3
evidence (titanic Sex column rejected 891/891 because
`identity.person.gender`'s enum is case-sensitive; airports timezone
column rejected 6761/7698 because `datetime.offset.iana`'s enum has
12 entries vs ~600 IANA zones; titanic Name rejected 177/891 because
the regex disallows parentheses) demonstrates that **validators are
not always orthogonal ground truth**. Three of four worked examples
showed the validator broken while the model's prediction was correct
— a naive validator-retrieval module would mis-attribute these as
model errors and feed phantom signal into E01a, triggering retrains
on errors the model didn't make.

The decision shapes how the inference module signals what the
"correct" type is when the model and the validator disagree.

## Considered Options

- **Option A — From-scratch validator-retrieval.** Run all 240
  validators, pick max pass-rate. Simple, fast, deterministic. Fails
  the Sex/timezone/Name evidence: when the validator is the broken
  signal, validator pass-rate alone routes the column to a wrong
  inferred type (or to `representation.text.string` fallback) and
  E01a gets junk signal.
- **Option B — Triangulator over multiple signals.** Fuse validator
  pass-rate with header-name match (Phase 1) and reserve
  generator-shape and sibling-context for Phase 2. Use the model's
  prediction as a structural prior: the cascade rules distinguish
  validator-broken (`validator_widening`) from model-wrong-subtype
  (`format_diversity_path_b`) from model-wrong-family
  (`misclassification`). The mechanism IS the discrimination axis.
- **Option C — Confirm-or-deny binary.** Module's only job is to
  ratify or contradict the model's prediction; output is
  `{confirm, deny, unsure}` plus a reason. Inference of an
  alternative correct type is out of scope; that lives in a follow-up
  bead. Useful for binary signal but cannot satisfy E01a's
  pair-distinctness threshold (which needs an `inferred` ID, not a
  confirm-or-deny verdict).
- **Option D — Validator audit first, inference second.** Split the
  bead. Phase 1: hand-audit + widen the broken validators (Sex enum,
  Name regex, IANA enum, utc_offset). Phase 2: build the inference
  module on cleaner validators. Cleaner foundation; substantially
  bigger scope; defers E01a unblock by an unknown timeframe.

## Decision Outcome

Chosen option: **Option B — triangulator over multiple signals**,
because it is the smallest architecture that distinguishes the three
failure modes the interview evidence surfaced
(validator-broken / subtype-drift / model-error) without depending on
a taxonomy audit (Option D), and because it produces an inferred
type ID that E01a's pair-structure can consume (Option C cannot).

The triangulator's load-bearing claim is that **validator pass-rate
and header-name match are independent error sources**. When they
agree, confidence is high; when they disagree, the disagreement is
itself a structural signal that the cascade rules (MADR 0081)
encode. Header-match is weighted higher than validator pass-rate
(`w_v=0.4, w_h=0.6`) because the interview evidence establishes
validators as the more error-prone of the two. Phase 1 ships with
just these two signals; the module's source MUST NOT reference
generator-shape or sibling-context (see MADR 0083 for the
phase-1 lock rationale).

### Consequences

- Good, because the cascade encodes domain knowledge that pure
  validator-retrieval cannot: when validator rejects but header
  confirms, the mechanism is `validator_widening` (a signal to
  E04, not to E01a's retrain queue).
- Good, because the triangulator naturally aligns with MADR 0075's
  existing 7-token rule cascade for `validate-corpus`. Vocabulary
  alignment (MADR 0081) means this module's output is consumable
  by the same downstream report machinery that 0075 ships, and
  the rule predicates are reusable scaffolding.
- Good, because Phase 1's two signals are cheap (240 validator
  scans + one taxonomy-label tokeniser) and fit comfortably in the
  ac-05 100ms budget on M1.
- Bad, because two-signal fusion introduces a weight choice that
  the implementation locks at `w_v=0.4, w_h=0.6` based on
  interview evidence rather than empirical sweep. If the locked
  weights underperform on the held-out floor (ac-02), the
  remediation is a separate decision (MADR amendment), not a
  weight tweak by the implementing agent.
- Bad, because the cascade adds rule-ordering complexity that
  Option A doesn't have. Mitigated: the cascade order is encoded
  as a `RULES: &[RuleFn]` slice with one unit test per rule, the
  same form MADR 0075 uses.
- Neutral, because Option D's premise (validator audit produces a
  cleaner foundation) is not refuted; it remains an open follow-up
  card if the inference module's outputs accumulate enough
  `validator_widening` flags to justify it.

References:
- `orbit/specs/2026-05-04-autonomous-type-inference/spec.yaml` (v1.3)
- `orbit/specs/2026-05-04-autonomous-type-inference/interview.md` (Q3)
- `orbit/decisions/0075-mechanism-bucket-coalesce.md`
- `orbit/decisions/0081-mechanism-vocabulary-aligned-with-madr-0075.md`
- `orbit/decisions/0083-phase-1-signal-scope-lock.md`
