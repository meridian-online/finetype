---
status: accepted
date-created: 2026-04-21
date-modified: 2026-04-21
---
# 0059. Demotion guard over validator-authoritative promotion

## Context and Problem Statement

MADR 0058 (do-not-promote-v17) surfaced a pipeline gap: `http_method`
and `excel_format` eval columns return generic
`representation.discrete.categorical` when every sampled value matches
a named type's validator. Its recommended follow-up was an explicit
"validator-authoritative promotion" Sharpen rule — a
`categorical → named_type` step to lift the prediction after a
validator match.

Discovery
`orbit/specs/2026-04-21-validator-signal-attribution/findings.md`
falsified that framing. Three findings govern this decision:

1. **Silent-zeros hypothesis FALSE.** The multi-branch model's
   validation branch receives real features at inference — the CLI's
   `cmd_profile` plumbs a compiled `Taxonomy` through
   `classify_multi_branch → compute_validation_tensor`. Not a
   plumbing bug.
2. **Validator precision pollution.** On `http_method` values
   (GET/POST/...), 25+ types return pass_rate = 1.000 in the
   240-dim validation feature vector — `comma_separated`,
   `postal_code`, `longitude`, `country`, `city`, ... The expected
   type `http_method` is ranked 25/240 tied with 24 others. The
   validation branch has no feature-level way to prefer http_method.
   It earns its keep only where validators are precise:
   `country_code` (ISO enum, ablation drops 0.994 → 0.328) and
   `email` (regex, ablation flips to `username`).
3. **Post-processing demotes correct predictions.** Raw multi-branch
   on `http_method` returns `technology.internet.http_method` (0.595);
   the full CLI pipeline returns `representation.discrete.categorical`
   (0.373). For `excel_format`, raw multi-branch returns `text.word`
   (already generic); `disambiguate_categorical` at
   `crates/finetype-model/src/column.rs:3881` then demotes
   `text.word → categorical` because its guard "top is generic + 3-20
   unique short non-numeric values" matches exactly.

The failure pattern is `named_type → categorical` demotion, not
`categorical → named_type` omission. The originally-proposed
promotion rule would layer a hack on top of the existing validation
branch architecture rather than address the actual cause.

## Considered Options

- **A. Validator-authoritative promotion rule (MADR 0058's follow-up).**
  Add a Sharpen step that lifts `categorical` predictions to a named
  type when every sampled value passes that named type's validator.
  **Rejected**: layers on top of an architecture that already has a
  validation branch, doesn't address the upstream demotion, and opens
  a precision failure mode (25+ types pass on http_method values,
  the rule would pick an arbitrary one).
- **B. Fix the validation branch's precision via validator audit +
  retrain.**
  The 25-way pass_rate tie is a training-data problem —
  many type validators are functionally no-ops for short strings
  (`^.+$`, alphanumeric-permissive patterns). Tightening them and
  retraining raises the validation branch's ceiling system-wide.
  **Deferred**, not rejected. Retrain-adjacent, larger lever,
  correct but expensive. Park until after the Sharpen-layer fix
  ships and measures whether residual errors are Sharpen or branch.
- **C. Demotion guard in `disambiguate_categorical`.**
  Prevent Sharpen from demoting a named-type prediction when the
  validator confirms it AND the validator is precise (enum-constrained
  OR regex with anchored, non-permissive body). Narrow, no-retrain,
  Sharpen-layer-only fix. Specifically targets the demotion pattern
  the evidence names.

## Decision Outcome

Chosen option: **C**. Add a validator-confirmed demotion guard to
`disambiguate_categorical`. Reject the promotion framing (option A).
Park the validator audit (option B) as a future retrain-adjacent spec.

The guard's predicate is:
```
validation_exists(current_label)
  AND validation.is_precise()
  AND every !s.trim().is_empty() value passes validator.is_valid()
```

"Precise" is defined as enum-constrained OR regex whose anchored
body is not in the rejected-permissive-pattern set (`^.+$`,
`^[A-Za-z0-9_]+$`, `^\S+$`, and similar loose patterns). The full
predicate and rejected set are specified in the spec's ac-01;
enforced against the real taxonomy via an audit (ac-01b).

### Rollback contingency

If the full-eval regression gate (spec ac-05) shows
`regressions > 0`, the predicate is tightened — additional permissive
patterns blacklisted in `is_precise()` — and the eval is re-run. If
no tightening reaches `regressions == 0` without losing the
`excel_format` fix (ac-04), the spec is paused and a follow-up
discovery card is opened. This decision moves to `rejected` if that
contingency fires; only moves to `accepted` after ac-05 verifies
zero regressions.

### Consequences

- Good, because the fix is narrow, Sharpen-layer, no retrain
  required. Ships within one PR.
- Good, because it targets the actual observed failure pattern
  (`named_type → categorical` demotion) rather than a post-hoc
  promotion workaround.
- Good, because the rejected-pattern audit (ac-01b) surfaces every
  taxonomy entry that would NOT benefit from the guard, feeding
  future validator-precision work (option B).
- Bad, because `http_method`'s demotion path is different (likely
  pre-Sharpen sibling-context enrichment) — this decision does NOT
  fix http_method. Evidence is documented in spec ac-06; follow-up
  card required for that case.
- Bad, because the precise-predicate is a string-equality check
  against a checked-in rejected list. A loose pattern not yet in
  the list will slip through; ac-01b's audit catches these at PR
  time but not at taxonomy-edit time. Future tightening (e.g. parse
  the regex and compute its implied character class) is possible
  but out of scope.
- Neutral, because the validator precision audit (option B) is
  parked, not lost — a clean separation between "don't demote what
  the validator confirms" (this spec) and "make the validator
  itself precise enough to be worth confirming" (future v18-adjacent
  work).

## References

- Discovery:
  `orbit/specs/2026-04-21-validator-signal-attribution/findings.md`
- Prior decision (context): `orbit/decisions/0058-do-not-promote-v17-relabel-scale-too-small.md`
- Prior decision (http_method enum-only framing):
  `orbit/decisions/0051-http-method-enum-only.md`
- Spec: `orbit/specs/2026-04-21-sharpen-demotion-guard/spec.yaml`
- CLAUDE.md Precision Principle: *"A validation that confirms 90% of
  random input is not a validation."*
