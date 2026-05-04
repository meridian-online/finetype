---
status: accepted
date-created: 2026-05-04
date-modified: 2026-05-04
---
# 0081. Inference mechanism vocabulary aligned with MADR 0075

## Context and Problem Statement

The inference module (MADR 0079) outputs a `mechanism` token per
column that classifies why the model's prediction differs from
the inferred truth. MADR 0075 already defined a 7-token
rule-emitted vocabulary for `validate-corpus`'s mechanism
cascade: `enum_overfit`, `format_diversity_path_a`,
`format_diversity_path_b`, `code_vs_canonical_path_a`,
`code_vs_canonical_path_b`, `misclassification`, `fallthrough`.
The 4-bucket display roll-ups (`format_diversity`,
`code_vs_canonical`) are explicitly NOT emitted per MADR 0075's
rule-owned-trigger doctrine. The bead's interview originally
proposed a parallel 4-tag vocabulary (`header-signal /
value-shape / prefix-shape / sibling-context`), but four
review-spec iterations established that the inference module is
the *forward direction* of MADR 0075's cascade: same
discrimination axis, different argument shape. Two
parallel vocabularies for the same axis would invite drift.

## Considered Options

- **Option A — Use the bead's original 4-tag vocabulary.**
  `header-signal`, `value-shape`, `prefix-shape`,
  `sibling-context`. Independent of 0075's vocabulary. Captures
  the *signal source* axis the inference triangulator uses, but
  doesn't match the *failure-mode* axis 0075 captures. Two
  parallel systems; downstream tooling has to reconcile.
- **Option B — Align with MADR 0075's rule-emitted tokens, plus
  triangulator-specific extensions.** Emit the 7 tokens 0075
  defines (rule-emitted, with path_a/path_b suffixes preserved)
  plus three new tokens that 0075's cascade structurally cannot
  emit: `validator_widening` (predicted == inferred AND
  validator rejects AND header confirms; only the triangulator
  sees the header signal); `prediction_confirmed` (predicted ==
  inferred AND validator passes; 0075 only runs on rejects);
  `unknown_no_fit` (no candidate scored above
  `fallback_threshold`; 0075 has `fallthrough` for the empty case
  but not the all-low-score case). 10 tokens total.
- **Option C — Hybrid: both axes recorded.** `mechanism` column
  = 0075 bucket; new `signal_axis` column = bead's 4 tags. Two
  columns. Maximum information; widens the schema; downstream
  consumers must read two columns to triage.

## Decision Outcome

Chosen option: **Option B — align with MADR 0075's rule-emitted
tokens plus three triangulator-specific extensions**, because
the inference cascade and the validate-corpus cascade share
predicate structure (same `predicted == inferred` /
`broad-prefix match` / `XOR allowlist` checks); using one
vocabulary means one downstream report machinery, one
cross-cycle attribution table, and one place to check when
auditing failure modes.

The 10-token closed set is:

**From MADR 0075 (rule-emitted, suffixed):**
- `enum_overfit`
- `format_diversity_path_a`
- `format_diversity_path_b`
- `code_vs_canonical_path_a`
- `code_vs_canonical_path_b`
- `misclassification`
- `fallthrough`

**Triangulator-specific (new):**
- `validator_widening` — predicted == inferred AND predicted's
  validator rejects ≥50% AND header_match for predicted ≥0.7
  AND argmax == predicted. The header signal is the load-bearing
  evidence that 0075's cascade structurally cannot see.
- `prediction_confirmed` — predicted == inferred AND validator
  pass-rate ≥0.7. 0075 doesn't emit this because 0075 only runs
  on B01-detected rejects.
- `unknown_no_fit` — `max_score < fallback_threshold` (= 0.4);
  inferred = `representation.text.string`, confidence = 0.3.
  0075's `fallthrough` covers the empty-input case; this covers
  the all-low-score case.

The display roll-ups `format_diversity` and `code_vs_canonical`
are NEVER emitted by either cascade. They exist only as analyst
report-table aggregates over the suffixed forms (per MADR 0075's
"trigger label is rule-owned" doctrine).

### Consequences

- Good, because validate-corpus's existing reports and the
  inference module's new outputs share one vocabulary; an analyst
  reading either source sees the same tokens with the same
  semantics.
- Good, because the rule predicates that 0075 already encodes
  (broad-prefix match, XOR allowlist, enum-vs-pattern detection)
  are reusable by the inference cascade. Implementing agent
  considers extracting a shared cascade primitive (see spec
  implementation_notes).
- Good, because the three new tokens are clearly delimited from
  the 0075 set. Future readers see "if it has `path_a/path_b`
  suffix it's MADR 0075; if it's one of the three new ones it's
  triangulator-specific." No ambiguity.
- Bad, because the bead's original 4-tag vocabulary
  (header-signal/value-shape/prefix-shape/sibling-context)
  captured a useful orthogonal axis (signal source). That axis
  is partially recoverable from the `signals` JSON the module
  emits per column — analysts who need it can extract from
  `validator_pass_rate` vs `header_match` magnitudes — but it's
  no longer a first-class column.
- Bad, because the cascade's rule order (spec ac-09) couples the
  inference module to 0075's rule-cascade order. If 0075 ever
  reorders rules, the inference module must reorder
  correspondingly. Mitigated: both cascades share the
  rule-owned-trigger doctrine, so reordering would surface in
  PR review against 0075 directly.
- Neutral, because the empty-samples case is now a precondition
  guard (structurally before the cascade) rather than a Rule N
  inside it. This is a structural difference from 0075 (which
  has `fallthrough` as a terminal rule) — the triangulator
  cannot enter the cascade without samples to score, so the
  precondition is the architecturally correct placement.

References:
- `orbit/decisions/0075-mechanism-bucket-coalesce.md` (the
  source vocabulary)
- `orbit/specs/2026-05-04-autonomous-type-inference/spec.yaml`
  (ac-09 cascade order, constraint block)
- `orbit/reviews/finetype-7zi/review-spec-2026-05-04.md` (HIGH 1
  surfaced the unsuffixed-vs-suffixed token mismatch in v1.0)
