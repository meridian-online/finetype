# v16 N=1 email regression — investigate (interview)

**Status:** interview (investigation pending)
**Created:** 2026-04-20
**Severity:** low (edge-case, not a user-common path)
**Related:** decision 0049, v0.6.17 release PR
**Next step:** `/orb:discovery` to reproduce + locate the cause, then `/orb:spec` once a fix direction is chosen (rule vs retrain)

## Context

v16 regresses against v14 on a narrow case: classifying a single-value
email column. With one value in column mode, v16 returns `plain_text`
where v14 returned `identity.person.email`. At N=5 (and higher) v16
classifies email correctly. IPv4 and URL do NOT regress at N=1.

Surfaced by the v0.6.17 release smoke tests — four assertions failed
on the same root cause. Workaround: updated those assertions to use
URL (still reliable at N=1) or 5-value email columns (the realistic
CLI usage anyway).

## Expected behaviours

1. Reproduce: `finetype infer -i "john@example.com" --mode column`
   returns `identity.person.email` with reasonable confidence (v14
   behaviour), not `representation.text.plain_text`.
2. Investigation documents WHY v16 regressed while v14 didn't.
   Plausible hypotheses:
   - v16's expanded synthetic blend (ssn/http_method/cpt/loinc short
     ASCII tokens — decision 0049) shifted the model's prior at low
     sample sizes.
   - Different dropout mask at save time changed which email features
     the model emphasises.
   - Training data mix ratio or augmentation rate changed the email
     class's effective weight.
3. Fix applied via ONE of:
   - a value-based sharpen rule (R32?): if all column values match
     `^\S+@\S+\.\S+$`, snap to `identity.person.email` regardless of
     model confidence. Decision 0048 compliant.
   - a retraining change (data blend tweak, loss weight) that makes
     v17/v16.1 reliable at N=1 without the rule.
4. Smoke test regression-lock: restore the N=1 email assertion with
   a comment noting the regression is fixed and the baseline.

## Non-goals

- NOT blocking v0.6.17 — ship the +2 eval improvement now.
- NOT adding header-based disambiguation (decision 0048 says no).
- NOT a general "robust at N=1" campaign. Just email.

## Notes

- URL and IPv4 both still classify correctly at N=1. The regression
  is specific to email. Worth understanding why.
- Investigation likely fits in a `/orb:discovery` session — compare
  v14 vs v16 model output layers for `john@example.com`, look at
  per-branch scores, identify which branch changed most.
