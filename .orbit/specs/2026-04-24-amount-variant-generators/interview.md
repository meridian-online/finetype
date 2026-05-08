# Discovery: Amount-Variant Generators

**Date:** 2026-04-24
**Interviewer:** Nightingale
**Card:** .orbit/cards/0002-semantic-type-detection.yaml
**Mode:** discovery

---

## Context

v18 retrain HELD (decision 0062). Per-column diff surfaced 11 amount
subtypes that collapse to plain `finance.currency.amount` in both v16
and v18:

- `amount_accounting`, `amount_apostrophe`, `amount_code_prefix`,
  `amount_comma`, `amount_comma_suffix`, `amount_crypto`, `amount_lakh`,
  `amount_multisym`, `amount_neg_trailing`, `amount_nodecimal`,
  `amount_space`.

The v18 handover frames this as a "needs per-subtype generators"
backlog item, but **the generators already exist** in
`crates/finetype-core/src/generator.rs` lines 3835–3965. So the gap
is upstream or downstream of generation — not generation itself.

That reframing is the first discovery finding and shapes every question
below.

## Q&A

### Q1: Failure-mode hypothesis
**Q:** Generators exist for all 11 amount subtypes. v16 and v18 both
collapse these to plain `amount`. What's the actual failure mode?
**Options presented:** (a) training volume/imbalance, (b) value-shape
overlap between subtypes, (c) no real-world distilled data, (d) not
sure — measure first.
**A:** (d) Not sure — need to measure first.

**Consequence:** This discovery produces a **measurement spec** with a
decision gate. Remediation is data-led, not committed up-front.

### Q2: Diagnostics to run
**Q:** What measurement artefacts would diagnose the failure with
confidence? (Multi-select.)
**Options presented:** corpus count per subtype; pairwise value-shape
overlap (char-class signature Jaccard); confusion matrix on eval;
per-subtype confidence distribution.
**A:** Implementation detail — drive full-auto.
**Nightingale's call (recorded):** All four. They are cheap, independent,
and complementary — no reason to skip any. The spec's ac-01 through
ac-04 will be one AC per diagnostic.

### Q3: Coverage priority
**Q:** Which subtypes matter most? Tier the cluster, lift all 11
equally, or let data decide?
**A:** Follow the data — measurement picks priority. Confusion matrix +
real-world corpus availability determine which subtypes get the
remediation effort.

### Q4: Spec scope — measurement only vs measurement + remediation
**Q:** Should this spec be measurement-only (with a MADR recommending
next steps), or measurement + remediation in one spec?
**A:** Measurement + remediation in one spec.

**Consequence:** AC list is conditional — the remediation ACs are
written as "address the diagnosed mechanism" rather than naming a
specific fix up-front. Spec will enumerate the decision matrix
(imbalance → rebalance; overlap → tighten generator output; confident-
but-wrong → value_sharpen rule; flat-confidence → eval-set enrichment).

### Q5: v19 gating
**Q:** Does v19 retrain block on this discovery, run in parallel, or
consume its output?
**A:** v19 blocked on this. No retrain sweep until diagnosis +
remediation ship.

**Consequence:** Spec carries a hard constraint: `v19 sweep scripts and
runs are forbidden until this spec ships`. Enforced by a header comment
in `scripts/sweep_v19.sh` (when it lands) and by sprint policy.

### Q6: Success gate
**Q:** When do we say remediation shipped and v19 is unblocked —
measurable eval lift, mechanism verified, or both?
**A:** Both — mechanism fix + v19 smoke baseline.

**Consequence:** Spec ships (i) mechanism-verified fix (post-fix
diagnostic artefact demonstrably shows the mechanism addressed —
e.g., balanced corpus counts, reduced Jaccard overlap, etc.), plus
(ii) a single-seed v19 smoke eval as directional signal before a full
v19 sweep commits.

### Q7: Off-limit areas
**Q:** Constraints — what's off-limits? Taxonomy, plain `amount` type,
real-world data, training infra?
**A:** None off-limits, as long as every touched area is tabled in the
spec. All four are explorable.

**Consequence:** No hard "do not touch" constraints. Instead, spec will
enumerate the four areas (taxonomy / plain-amount generator / real-data
sourcing / training infra) with an explicit flag per area: `touched:
yes|no` at spec-authoring time, adjustable at implementation if
diagnosis points to one of them.

### Q8: v19 smoke-eval scale
**Q:** What scale for the v19 smoke run? 1×50ep, 1×100ep, 3×100ep, or
decide at spec time?
**A:** Implementation detail — drive full-auto.
**Nightingale's call (recorded):** **1 seed × 100 epochs** (≤150 min).
Directly comparable to v18 per-seed numbers (v18 seed 42 = 0.9134 at
100ep), good signal/cost ratio, and if the smoke shows a clear lift
Hugh can choose to promote to a full 3-seed sweep inside a separate
v19-proper spec. A 50-epoch smoke risks conflating "didn't converge"
with "didn't help"; a 3-seed smoke is overkill for a directional check.

---

## Summary

### Goal

Diagnose why 11 `finance.currency.amount*` subtypes collapse to plain
`amount` in v16/v18, apply the mechanism-verified remediation the
diagnosis prescribes, and unblock v19 retrain with a single-seed
smoke baseline demonstrating directional lift.

### Constraints (from interview — non-negotiable)

1. **Generators already exist for all 11 subtypes.** The spec does NOT
   write new generators from scratch; it measures what the existing
   generators produce and fixes whatever the diagnosis names.
2. **Remediation is data-led.** No commitment to a specific fix
   (rebalance / tighten generator output / new value_sharpen rule /
   eval-set enrichment / real-data sourcing) until the diagnostics
   surface a clear mechanism.
3. **v19 retrain is blocked until this spec ships.** No v19 sweep
   scripts, no v19 models, no v19 promotion while this is open.
4. **Success requires both mechanism-verified fix AND v19 smoke lift.**
   Either alone is insufficient — mechanism without lift means the
   mechanism hypothesis was wrong; lift without mechanism means we
   can't reason about what fixed it.
5. **Off-limit areas:** none. Taxonomy, plain-`amount` type, real-data
   sourcing, and training infra are all explorable, with each touched
   area tabled in the spec.
6. **Smoke scale is 1 seed × 100 epochs** — directly comparable to v18
   per-seed numbers, 2.5× cheaper than a full sweep.

### Success Criteria

1. Four diagnostic artefacts produced: corpus count per subtype,
   pairwise value-shape Jaccard, confusion matrix on the 11 eval
   columns, per-subtype confidence distribution.
2. Named mechanism in a MADR (one of: volume imbalance, value-shape
   overlap, confident-but-wrong classifier, flat-confidence under-
   signal, or an honest "no single mechanism — multiple interacting
   causes" with attribution percentages).
3. Remediation applied that addresses the named mechanism; post-fix
   diagnostic artefact demonstrates the mechanism is measurably
   reduced.
4. v19 smoke run (1 seed × 100 epochs) produces a per-subtype label-
   accuracy delta vs v16 baseline on the 11 eval columns. The delta
   must be **≥ +3 subtypes correctly predicted** (net, regressions
   subtracted) to unblock v19-proper.

### Decisions Surfaced

- **Framing correction:** "needs per-subtype generators" → "diagnose
  why existing generators' output isn't distinguishing subtypes".
  Record under MADR TBD-number at spec time; refines v18 handover.
- **v19 hard gate:** sprint-policy decision that v19 cannot start
  until this spec ships. Record under MADR TBD-number at spec time;
  complements decision 0062 (v18 HELD).
- **Remediation-in-same-spec:** measurement and fix ship together,
  with a conditional AC list keyed on diagnosis outcome. Accepted
  architectural pattern; may be documented as a MADR if the pattern
  recurs.

### Open Questions

None material. Downstream spec-author (spec stage) will:

- Assign MADR numbers for the two decisions above.
- Pin the **fixture** for the diagnostic runs (v18 FTMB artefact vs
  regenerated; the v18 `output/multibranch-training/v18.ftmb` was
  deleted post-sweep per v18 handover line 54 — may need regen).
- Choose the confusion-matrix harness (reuse profile eval, or a
  bespoke per-subtype batch call).
- Decide whether to table all 4 off-limit-question areas in the spec
  or just those the diagnosis implicates.

---

**Next step:** `/orb:spec` to generate a structured specification from
this discovery.
