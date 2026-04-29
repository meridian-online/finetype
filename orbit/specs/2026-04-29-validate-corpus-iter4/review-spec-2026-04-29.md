# Spec Review

**Date:** 2026-04-29
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-29-validate-corpus-iter4/spec.yaml
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 2 (both LOW) |
| 2 — Assumption & failure | not triggered | — |
| 3 — Adversarial | not triggered | — |

The spec describes a single-file YAML regex widening that composes a verbatim
canonical sibling pattern, gated by three new unit tests, an existing
fixture-lock test, and a documented before/after demonstration. Pass 1
turned up zero MEDIUM-or-higher findings and no deepening content signals
(no training-data, deployment, security, schema-migration, or
cross-system-boundary work — the follow-up card explicitly defers all
model-side work). Pass 2 / Pass 3 not warranted.

## Findings

### [LOW] Card 0014 specs[] already lists the iter-4 spec
**Category:** test-gap
**Pass:** 1
**Description:** ac-10's verification asserts that `git diff main` shows
`card 0014 specs[] append`. But `orbit/cards/0014-profile-validate-precision.yaml:62`
already contains `"orbit/specs/2026-04-29-validate-corpus-iter4/"`. Either
the card was updated in a precursor commit (in which case `git diff main`
won't show the line as added — verification phrasing is misleading), or
the spec author intended to verify that the line is **present** rather
than freshly appended. Either way the AC's success condition holds, but
the verification text doesn't match the file's current state.
**Evidence:** `orbit/cards/0014-profile-validate-precision.yaml:62` reads
`- "orbit/specs/2026-04-29-validate-corpus-iter4/"`.
**Recommendation:** Tighten ac-10's verification to "card 0014 specs[]
contains the iter-4 spec path (already added during spec drafting; verify
present)." No spec-level change required if the implement skill treats
the AC as "ensure present, don't fail if already there."

### [LOW] ac-08 follow-up card slot may already be claimed
**Category:** assumption
**Pass:** 1
**Description:** ac-08 specifies card index 0015 with a fallback "or the
next available NNNN slot if 0015 is already taken." The current highest
card index is 0014. The spec hedges correctly, so this isn't a defect —
just a reminder that index allocation should be confirmed at implement
time (e.g. `ls orbit/cards/` immediately before card creation), not
assumed from the spec snapshot.
**Evidence:** `ls orbit/cards/` shows 0001..0014; 0015 currently
unclaimed but not yet created in this branch.
**Recommendation:** None — spec already handles this with the fallback
clause. Noted for the implement skill so the index is double-checked
right before file creation rather than baked in via assumption.

---

## Honest Assessment

This spec is unusually tight. The change is a single regex alternation in
one YAML file, the borrowed pattern is byte-identical to a sibling type's
canonical regex (with documented MADR-backed precedent in flight as 0078),
the demo dataset is already on disk with a verifiable 100-row 63-reject
baseline, and the iter-3 fixture-lock test (`vci3_fixture_attribution_regression_match`)
exists at `crates/finetype-eval/src/bin/validate_corpus.rs:1678` exactly as
the spec claims. Three new `vci4_*` regression tests cover positive widening
(ac-02), preservation of existing alternations (ac-03), and continued
rejection of non-money tokens (ac-04). Misclassification deferral is
explicitly card-mandated by 0014's goal text — the spec respects scope
discipline rather than scope-creeping into model-side work.

The biggest residual risk is silent attribution drift on FIFA Value/Wage
rows (currently classified `code_vs_canonical / path-b-codetype`) — but
the spec correctly reasons that MADR 0076 makes these forward-looking
anchors that don't fail the test even if they silently start passing
validation. That's a thoughtful handle on what could otherwise have been
a hidden gotcha.

Ship it.
