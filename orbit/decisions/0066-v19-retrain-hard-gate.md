---
status: accepted
date-created: 2026-04-24
date-modified: 2026-04-24
complements: 0062
---

# 0066. v19 retrain hard gate

## Context and Problem Statement

Decision 0062 held v18 on the basis of net-zero promotion signal (47/55
v16 failures carried unchanged; per-column diff 8 fixes / 8 regressions /
47 persistent). The amount-variant collapse then turned out to be a
pipeline-layer mechanism (MADR 0065) — `header_hint()` at
`crates/finetype-model/src/column.rs` over-generalised on the `amount`
substring and forced every variant header back to plain
`finance.currency.amount`. Retraining was not the lever; the ac-06 fix
was a 12-arm edit to the hint table.

v19 retrain is therefore **sprint-eligible** but not **free-running**.
This MADR pins the hard gate for promoting any future retrain to
`models/default`.

## Considered Options

- Option A — no gate. Allow any v19 retrain to promote on seed-42 smoke.
- Option B — 3-seed sweep sufficient, no per-domain / per-column
  regression floor.
- Option C — 3-seed sweep + per-column diff + per-domain regression
  ceiling, matching the discipline that held v17 and v18.

## Decision Outcome

Chosen option: **Option C — 3-seed sweep + per-column diff + per-domain
regression ceiling**.

### Gate specification

A candidate model is eligible for promotion to `models/default` only if
**all** of the following hold against the current `models/default`
baseline (currently sherlock-v16) on the 448-row eval manifest:

1. **3-seed sweep completed**: seeds 42, 43, 44 × 100 epochs each, all
   three with `val_acc ≥ 0.912` (the AUTO_ACCEPT floor that v18 used —
   decisions 0060, 0062).
2. **Winner selection**: the highest-`val_acc` seed is the promotion
   candidate. Ties broken by lowest `val_loss`, then lowest epoch index.
3. **Full profile eval delta**: the candidate's full profile eval on the
   448-row manifest must pass both gates simultaneously:
   - `net_label_delta = (label_correct_post) - (label_correct_pre) ≥ +1`
   - `net_domain_delta = (domain_correct_post) - (domain_correct_pre) ≥ 0`
4. **Per-domain regression ceiling**: no domain (container, datetime,
   finance, geography, identity, representation, technology) regresses
   by more than **3** label-level hits relative to baseline (matching
   the v18 HELD limit — decision 0062).
5. **Per-column diff published**: a diff artefact (analogous to
   `orbit/specs/2026-04-21-v18-retrain/v16-v18-diff.md`) ships with the
   promotion PR, enumerating fixes / regressions / persistent-same /
   persistent-churn.
6. **Mechanism attribution**: if the candidate includes a pipeline-layer
   change as well as retrained weights, the PR description must name
   which change moved the numbers. This prevents a repeat of the v17
   confusion where training-data churn was credited for improvements
   that actually came from Sharpen-rule edits.

A candidate failing any of (1)–(6) is **HELD** in the manner of v17 and
v18 — branch retained for future reuse, no symlink flip, no HF publish,
no `FINETYPE_CI_MODEL` bump.

### Consequences

- Good, because the discipline survived v17 hold (decision 0054) and v18
  hold (0062) under stress and produced honest outcomes both times.
- Good, because tying the gate to measurable deltas (not narrative
  judgment) means any future author or agent can apply it deterministically.
- Bad, because 3-seed sweeps cost ~7.5h of compute — the minimum bar to
  measure anything.
- Bad, because per-domain regression ceiling of 3 is arbitrary; it is
  the value v18 tolerated at the gate limit (datetime +3). A future card
  may revisit if we collect enough evidence.

## Cross-references

- **Decision 0054** — v17 hold (same pattern: signal-to-noise below floor).
- **Decision 0062** — v18 hold (same pattern: net-zero delta).
- **MADR 0065** — amount-subtype collapse mechanism; explains why
  retraining alone was not the lever for this family of failures.
- **MADR 0067** — framing correction superseding v18 handover's
  "write per-subtype generators" guidance.
