# Cascade-rule precision via LLM-labelled training data

## Problem

The mechanism cascade (MADRs 0075 + 0081) classifies *why* FineType
got a column wrong, picking one of ten closed mechanism tokens.
ac-11's labelled_eval grading shows the cascade gets the token right
**95.2%** of the time at population scale — strong.

But ac-12's per-cell spot-check (22-gap sample, seed 20260520)
surfaced a systematic-looking weakness: **4 of 8 non-empty cells fail
the 90% per-cell precision threshold under my pre-screen reading**.
The failures share a single shape — the cascade reaches for a
specific mechanism token (`validator_widening`,
`code_vs_canonical_path_a`, `format_diversity_path_a`) when the
correct token is the generic `misclassification` or `unknown_no_fit`.
Concrete examples:

- `EMAIL` column containing full street addresses → cascade says
  `validator_widening`. But widening the email validator can't make
  `"42294 Foster Plaza West…"` a valid email. Correct token:
  `misclassification`.
- `URL` column containing bare integers → same `validator_widening`
  miscall.
- Chemistry atom-label column (`C1'`, `C2`, `C2'`) → cascade says
  `code_vs_canonical_path_a`. No code/canonical distinction here;
  these are a fixed enum. Correct token: `enum_overfit`.
- `WEIGHT (%)` column with values `-90.0`, `100.0` → cascade says
  `format_diversity_path_a`. Sense's `identity.person.weight` is
  fundamentally wrong (negative human weight?); correct token is
  `misclassification`.

This is **cascade-rule precision**, not lens-stack precision. The AND
filter (ac-09) correctly flags these columns as needing attention;
the cascade's mechanism *label* is off by one slot in the closed set.

## Why this matters

The mechanism token drives `recommended_action_class` (validator
widening vs model retrain vs training data addition vs taxonomy
addition vs fallback adjustment). When the token is wrong, the
recommended fix is wrong — which means the v20 retrain plan would
allocate training effort against the wrong improvement category for
those columns.

At 1% of the report (~390 of 64,565 corroborated gaps), the
financial impact for v20 is small. For v21 and beyond, as the
cascade is asked to discriminate finer-grained failure modes, the
precision floor matters more.

## Proposed approach

LLM-labelled training data for an improved cascade rule set:

1. **Seed**: fan out 5,000-10,000 corroborated gaps as sub-agent
   tasks. Each agent receives the gap context (sense_prediction,
   YDF prediction, sample_values, mechanism_token currently
   assigned) and outputs (a) the correct mechanism token and (b)
   confidence.
2. **Aggregate**: any gap where the LLM-labelled token differs from
   the cascade's assignment is a candidate disagreement. The 4 cells
   that failed ac-12 likely produce disagreement clusters.
3. **Diagnose**: cluster the disagreements by `(cell, cascade_token,
   llm_token)` to surface the systematic rule errors.
4. **Patch**: for each systematic error, add or refine a cascade
   rule in MADRs 0075 / 0081 territory. Re-grade against
   labelled_eval to confirm the population-precision floor (95.2%)
   doesn't regress.

## Cost shape

- 10k gaps × 1 LLM call each. At a small-model rate, this is dollars
  to tens of dollars, hours of wall clock with rate limits.
- Iteration: 2-3 cycles of (label → diagnose → patch → re-grade).
- Risk: LLM labels themselves carry epistemic noise; per-gap labels
  need a consensus rule (e.g. label twice, accept only on agreement)
  or a calibration step against the existing labelled_eval ground
  truth.

## Why not now

- m-19 is closing; this would expand scope.
- Cascade precision at 95.2% population is already past the
  v20-retrain gate. The 5% weakness concentrated in 4 cells is real
  but addressable in v21 prep.
- No infrastructure for LLM-fanout-with-aggregation yet — building
  it is its own piece of work.

## Filing

Card candidate: *"Cascade mechanism precision lift via LLM-labelled
training data"*. Pillar: agent self-learning (the cascade improves
its discriminations by consuming labelled disagreement data). Best
filed after m-19 closes; ac-12's outcome (the 4 demoted cells) is
the concrete payload this card would address.

## Context

This memo was filed 2026-05-23 during ac-12 pre-screen analysis of
the gittables multi-lens corpus diagnostic
(`.orbit/specs/2026-05-20-gittables-multi-lens-diagnostic/`).
The pre-screen artefact lives at
`eval/gittables/corpus_pass/spot_check_prescreen.md`.
