# Design interview — deterministic-layer audit

**Date:** 2026-06-10
**Context:** Emerged from a session that ran cheat-check → eval review → four shipped
fixes → 0.6.26 release. The author formed the view that the inference pipeline's
non-neural layer is "much, much more complex than I'd understood" and asked whether
it can be interrogated for correctness.

## What the quick pass already established (pre-interview evidence)

Traced all 449 columns across 32 eval files with `finetype profile -v`:
- The pipeline self-reports every non-neural decision in `-v` (header hint, feature
  rule, locale, veto hard/advisory). Per-column interrogation already exists.
- ~18 deterministic steps defined; on the eval suite, **6 of 7 feature rules and 6 of
  11 header hints never fire** — including the just-shipped `header_hint_coord_veto`.
- Veto: 20 hard + 28 advisory firings; mostly catching real mispredictions, but
  three hard vetoes (`gender` 0%, `npi` 32%, `price` 33%) demote columns that look
  correctly typed.
- column.rs > 10k lines; the layer accreted across ~6 decisions.

## Open decisions resolved (AskUserQuestion, 2026-06-10)

**Q1 — Audit scope.** → **Everything incl. validators.** Full deterministic layer
(~18 steps) PLUS a Precision-Principle review of all 240 taxonomy validators. The
broadest option: the dormant-rule findings live mostly in the in-model feature rules,
and the false-veto suspects implicate validator precision, so both layers are in
scope.

**Q2 — Fix authority.** → **Verdict-only.** The audit produces verdicts + removal
preconditions; every code change (removal, validator fix, false-veto fix) is a
separate, later, gated decision. Matches the header-hint precedent (a net-negative
step can still be load-bearing until the model covers it).

## Settled earlier in the session (not re-litigated)

- **Staged dynamic workflow**, gated by author review: Stage 1 (behavioural, no corpus
  passes) fans out per step; its ledger gates Stage 2 (targeted corpus ablation).
  Two workflow invocations with a review gate, not one monolith.
- **Ground truth** for net-value = the stable gated-YDF oracle (canonical, spec
  2026-05-26-ydf-validation-gate), via the corpus-honest-gate machinery.
- **Deliverable** = a one-page ledger the author can act on, not a dump (author-
  interaction pillar).

## Halt conditions

- No pipeline behaviour changes. The audit's only writes are throwaway trigger inputs
  and the ledger.
- Every "remove" verdict carries a precondition; none is acted on within the audit.
- Stage 2 corpus spend is gated on author review of the Stage 1 ledger.
