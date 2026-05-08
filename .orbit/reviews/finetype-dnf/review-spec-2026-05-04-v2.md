# Spec Review (v2)

**Date:** 2026-05-04
**Reviewer:** Context-separated agent (fresh session)
**Bead:** finetype-dnf
**Cycle:** 2 (drive_review_spec_cycle = 1 in metadata; this is the second review pass)
**Verdict:** APPROVE | REQUEST_CHANGES | BLOCK

**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 3 |
| 2 — Assumption & failure | content signals (numeric baseline carried from v1, conditional mitigation trigger, weight-discipline OR clause) + Pass-1 MEDIUM count | 3 |
| 3 — Adversarial | Pass-2 surfaced an empirical-anchor numeric error (the same failure mode MADR 0084 just retired) and a sequencing gap (memo without gate marker) — both structural | 1 |

## Diff vs v1 review

The bead's `acceptance_criteria` field has been substantially rewritten between v1 and v2 (the source card `0016-phase-2-triangulator-signals.yaml` is unchanged; the bead acceptance is the authoritative spec per the user's instruction). v2 adds three ACs that did not exist in v1 (the plausibility memo, the MADR 0085 reconciliation gate, the weight-discipline statement) and binds the falsifiability numbers v1 flagged. Mapping:

| v1 finding | v2 disposition |
|---|---|
| HIGH — ac-02 cliff lift not falsifiable | RESOLVED in ac-05: pinned to ≥10 pp absolute over Phase 1's 0.204 baseline + AUC ≥2× over baseline |
| HIGH — ac-01 ≥60% unbacked aspiration | PARTIALLY RESOLVED in ac-04: outcome (b) named modal; ac-01 plausibility memo (new) requires per-signal back-of-envelope before code lands |
| HIGH — sibling-context vs MADR 0083 conflict unreconciled | RESOLVED in ac-02 [gate]: MADR 0085 is a hard prerequisite, options (A)/(B)/(C) enumerated, must reconcile MADR 0083 lines 105-115 |
| MEDIUM — generator-shape latency uncharacterised | RESOLVED in ac-06 [gate]: cost-model paragraph required; if estimate > 30ms headroom, spec picks one of three named mitigations up front |
| MEDIUM — ablation methodology underspecified | RESOLVED in ac-07: bench_infer_floor.py --disable-signal harness; decisive_signal column added to sidecar; ≥60% concentration thresholds named |
| LOW — schema migration missing | RESOLVED in ac-08: inference_signals_v2.tsv (15-col) introduced; Phase 1 sidecar frozen per H08 invariant |
| LOW — calibrate/measure leakage path | RESOLVED in ac-03: weight selection MUST NOT look at measure half; (a) MADR-locked or (b) calibrate-only sweep |
| LOW — no [gate] markers | PARTIALLY RESOLVED: ac-02, ac-05, ac-06 carry [gate]; ac-01 and ac-04 do not (see findings below) |

The headline movement is real and substantive. The remaining findings are concentrated in three areas: a numeric-anchor error inherited from v1, gate placement, and a weight-discipline ambiguity.

## Findings

### [HIGH] ac-05 baseline AUC value 0.014 appears numerically incorrect — same MADR-0084 failure mode

**Category:** test-gap
**Pass:** 3
**Description:** ac-05 states the falsifiable cliff-lift criterion is "AUC over [0.5, 0.7] on the measure half ≥2× Phase 1's baseline of ~0.014 (trapezoid)". I cannot reproduce 0.014 from Phase 1's progress.md table. Trapezoid integration of `non_unknown_rate` over `threshold ∈ [0.5, 0.7]` with the three samples in progress.md (0.5 → 0.204, 0.6 → 0.203, 0.7 → 0.071) yields:

```
((0.204 + 0.203) / 2) · 0.1  +  ((0.203 + 0.071) / 2) · 0.1
= 0.02035 + 0.01370
= 0.03405
```

Two-point trapezoid using only the endpoints (0.204 at 0.5, 0.071 at 0.7) yields `((0.204 + 0.071)/2) · 0.2 = 0.0275`. Neither matches 0.014. The plausible source of 0.014 is using `(b − a)/2 = 0.1` as the width instead of `b − a = 0.2`, i.e. an off-by-2× error in the v1 review that has now propagated into the bead's gating AC.

If the true baseline is 0.034 (three-sample) or 0.0275 (two-sample), the "≥2×" target shifts from 0.028 to 0.055-0.068. The implementing agent reading ac-05 today would aim for 0.028, which a two-signal Phase-1 module already nearly clears at 0.0275 — meaning the AUC sidecar diagnostic could pass without Phase 2 doing any new work.

This matters because ac-05 is the **load-bearing falsifiable gate** — the AC explicitly designated by the bead description as "the falsifiable claim that the new signals carry information". A target that is set 2× too low fails to falsify Phase 2 against a null that already passes it. This is the same failure mode MADR 0084 retired ("a numeric AC '≥X% on dataset Y' that isn't anchored to a measurement survives review because it's internally consistent"); v2 has bound the AC to a measurement, but the measurement number itself is wrong.

A second concern in the same AC: Phase 1's per-threshold curve in progress.md is a "deterministic sample of 1000 rows each", not the full 10,660-row measure half ac-05 asks Phase 2 to run on. Even after the trapezoid fix, the baseline must be re-derived from the same partition (full 10,660 rows) to be apples-to-apples. The 1000-row sample's @0.7 rate is 0.071; the full measure half's @0.7 is 7.1% per progress.md headline, so they likely agree, but the 0.5 and 0.6 samples have not been reported on the full partition.

**Evidence:**
- Bead ac-05: "AUC over [0.5, 0.7] on the measure half ≥2× Phase 1's baseline of ~0.014 (trapezoid)"
- `.orbit/specs/2026-05-04-autonomous-type-inference/progress.md:163-172` — the 1000-row sample table
- `.orbit/choices/0084-ac02-floor-empirical-recalibration.md:141-150` — the methodology lesson the bead is meant to honour
- v1 review `review-spec-2026-05-04.md:34` — first appearance of "0.014" without a derivation

**Recommendation:** Two corrections, in order:
1. Recompute the baseline against the full measure half (10,660 rows) at thresholds {0.5, 0.6, 0.7} before the spec drafts. Record the recomputed three-sample AUC in ac-05 verbatim, replacing "~0.014".
2. If the recompute is out of scope for spec drafting, lock the baseline to the simpler "non_unknown@0.5 ≥ 0.304" criterion (already in ac-05 as the primary metric) and demote AUC from a target to a sidecar diagnostic with no numeric pass-criterion. The ≥10 pp absolute lift is the un-ambiguous test; the AUC line is secondary and should not gate while its baseline is wrong.

Recommend (1) — it preserves the AUC-as-information-density check the bead authors clearly intended and removes a footgun for the implementing agent.

### [HIGH] ac-03 leaves weight-discipline open as (a) OR (b) — spec does not pre-commit

**Category:** missing-requirement
**Pass:** 2
**Description:** ac-03 says weights are selected via "either (a) design-time lock per a Phase 2 MADR mirroring MADR 0079's discipline, OR (b) sweep on calibrate half ONLY with measure-half number computed once at locked weights." Both options have integrity (no measure-half leakage) and the AC requires the choice plus rationale to be documented in spec. But the bead acceptance — which the user has named as the authoritative spec for this review — does not pick one.

This matters more than the v1 LOW finding about leakage path (which v2 resolved — leakage IS firewalled in both branches). The remaining concern is **decision discipline**, not leakage:

- Branch (a) is a forward-MADR commitment: a separate MADR is authored alongside the spec, with first-principles reasoning for each weight, like MADR 0079 did for w_v=0.4, w_h=0.6. The implementing agent never runs a sweep.
- Branch (b) is an empirical sweep: the implementing agent runs a calibrate-half grid search and locks weights from the result. Risk surface is wider (sweep grid, stop criterion, tie-breaks).

These have different review surfaces, different deliverables, and different risk profiles. Picking at spec time vs implementation time changes which MADRs need to exist and what the gate on ac-04 measures (ac-04 reports "non_unknown@0.7 at locked Phase 2 weights" — but the locked weights are different artefacts under (a) vs (b)).

Phase 1's MADR 0079 picked (a) explicitly. Phase 2 inherits the architecture and should inherit the discipline by default unless the bead authors believe Phase 2's signals are too unfamiliar for first-principles reasoning, in which case (b) is acceptable but should be named.

**Evidence:**
- Bead ac-03: "either (a) design-time lock... OR (b) sweep on calibrate half ONLY"
- `.orbit/choices/0079-triangulator-architecture-for-autonomous-inference.md:92-97` — Phase 1 chose (a) with rationale
- `.orbit/choices/0083-phase-1-signal-scope-lock.md:75-77` — extends weight invariant to 4 signals if Phase 2 ships, but does not pick a discipline
- ac-04 outcome (a)/(b) language pivots on "locked Phase 2 weights" — ambiguous which artefact

**Recommendation:** Pick at the bead acceptance level, not later. Recommended posture: (a) — Phase 2 MADR locks weights at design time, mirroring MADR 0079's discipline. Concrete starting point for spec discussion (not a binding pre-commit, but a useful default to argue against): keep w_v + w_h together = 0.5 of mass (Phase 1 had 1.0; halve to make room), allocate the other 0.5 between generator-shape and sibling-context based on which signal is hypothesised to recover the larger fraction of cliff-cases per the ac-01 plausibility memo. If the memo concludes neither signal can recover ≥25 pp alone, the rationale supports a roughly even 0.25/0.25 split between the new pair.

If the bead authors prefer (b), the AC should say so and add: explicit grid (e.g. {0.1, 0.2, 0.3} per weight subject to sum=1.0), explicit stop criterion (highest non_unknown@0.7 on calibrate half, ties broken by lowest standard deviation of confidence), and explicit single-number measure-half report.

### [MEDIUM] ac-04 not gated despite being the empirical floor decision

**Category:** constraint-conflict
**Pass:** 1
**Description:** v1 review's LOW finding about [gate] markers has been mostly addressed — ac-02 (MADR 0085), ac-05 (cliff lift), ac-06 (latency) all carry [gate]. But ac-04, the binary "Phase 2 ships vs Phase 2 documents structural ceiling" outcome decision, is NOT gated.

The argument for leaving ac-04 ungated: outcomes (a) and (b) BOTH ship the bead — there is no failure path that closes finetype-dnf without progressing. ac-05's cliff-lift gate is the falsifiable pass/fail; ac-04 is descriptive (which branch happened).

The argument for gating ac-04: it is the AC that produces the deliverable — either the spec amendment restoring autonomous-type-inference's ac-02 to 60%, or the next-after-0085 MADR documenting structural cause. Without the gate, the implement skill can't block bead-close on the deliverable being authored. The bead could close with ac-05 passed but neither the spec amendment nor the structural-cause MADR written.

This isn't a fatal omission — ac-05's gate carries most of the falsifiability load — but ac-04 carries the *narrative* deliverable. Recommend gating it.

**Evidence:**
- Bead ac-04: outcomes (a)/(b) both ship; deliverable is either spec amendment or new MADR
- Bead ac-05 [gate]: the falsifiable claim — already covers the empirical falsification axis
- Phase 1 spec.yaml line 498: ac-04, ac-05, ac-06 carried gates in Phase 1 — the analogous Phase 2 gate placement should be ac-02, ac-04, ac-05, ac-06

**Recommendation:** Add `[gate]` to ac-04. Rationale to record in the bead acceptance update: "ac-04 produces the bead's narrative deliverable (either ac-02 restoration to autonomous-type-inference or next-after-0085 structural-ceiling MADR); without a gate, bead-close can occur with the deliverable unauthored."

### [MEDIUM] ac-01 plausibility memo not gated — sequencing not enforced

**Category:** test-gap
**Pass:** 2
**Description:** ac-01 requires a Phase 2 plausibility memo "BEFORE any signal-implementation code lands". Without [gate] marker, the implement skill cannot enforce ordering — an implementing agent could reasonably ship signal code and back-fill the memo, which inverts the whole point of the AC (the memo is supposed to inform the weight discipline of ac-03 and the modal-outcome framing of ac-04).

The memo also has no falsifiable pass criterion. The text says "If neither alone reaches ~25 pp lift, the modal expectation is outcome (b) of ac-04" — but the memo passes regardless of whether the lift estimate is 1 pp or 50 pp, as long as the back-of-envelope is shown. So the memo's content can pass without informing the design choices it is meant to inform.

**Evidence:**
- Bead ac-01: "Authored BEFORE any signal-implementation code lands"
- Bead ac-03: weight discipline must be picked, but no link to ac-01's memo output
- Bead ac-04: modal-expectation framing pivots on memo's quantitative claim
- `parse-acceptance.sh` output: ac-01 row has `is_gate=0`

**Recommendation:** Either:
1. Mark ac-01 as `[gate]` so the implement skill blocks signal-code commits until the memo file exists. Pair with a path constraint ("memo lives at .orbit/specs/<phase-2-spec-dir>/plausibility.md or progress.md section §X").
2. Re-formulate ac-01 as a soft prerequisite that ac-03's "rationale documented in spec" must explicitly cite, so the memo's per-signal numbers feed the weight choice. Then ac-03 effectively gates ac-01 by content.

(1) is cleaner. (2) is feasible if the bead authors don't want a fourth gate.

### [MEDIUM] ac-06 mitigation trigger is conditional — implementing agent's cost estimate is not audited

**Category:** failure-mode
**Pass:** 2
**Description:** ac-06 says "if estimated cost > 30ms remaining headroom (Phase 1 p50 = 70ms), spec picks one mitigation up front". This is a conditional trigger — if the implementing agent estimates 25ms, no mitigation is required. But the *estimate itself* is not verified before commit. The actual cost is only measured at the 1000-column M1 benchmark, which fires at the latency-budget gate, after implementation.

Failure scenario: implementing agent under-estimates cost (e.g. claims 25ms, actual is 45ms), commits without mitigation, latency benchmark blows the budget at end-of-bead. The ac-06 [gate] catches it but the rework cost is high — picking a mitigation post-hoc requires re-architecting the signal extraction.

v1 review's MEDIUM finding here recommended a cost-model paragraph; v2 has that. The remaining gap is auditability of the estimate itself. Phase 1's spec.yaml line 443 had a concrete cost computation (240 × 8 × ~1µs ≈ 2ms); Phase 2's bead acceptance asks for a "paragraph" but doesn't pin the components (240 candidates, K samples, per-generator cost in µs).

**Evidence:**
- Bead ac-06: "240 candidate types × K samples × per-generator cost. If estimated cost > 30ms remaining headroom..."
- Phase 1 spec.yaml:443 — "240 × 8 × ~1µs ≈ 2ms" — concrete, auditable
- `crates/finetype-core/src/generator.rs` — 6843 lines (per v1 review's wc)

**Recommendation:** Tighten the cost-model paragraph to require:
- A concrete K value (Phase 1 used 8 samples — Phase 2 should pick K explicitly).
- A concrete per-generator µs estimate, with a method (e.g. "5 generators benchmarked from generator.rs averaged 80µs each at K=8 ⇒ 240 × 80µs = 19.2ms estimate").
- Mitigation chosen up front if estimate × 1.5 (safety margin) exceeds 30ms. The 1.5× margin guards against the under-estimation failure mode without forcing mitigation when there is genuine headroom.

### [LOW] ac-08 — cascade rule additions for new signals not enumerated

**Category:** missing-requirement
**Pass:** 1
**Description:** ac-08 says "Cascade gains rules for new signals; Phase 1 cascade rules unchanged." Phase 1 has 10 cascade rules in priority order (ac-09 of the autonomous-type-inference spec, "10-element closed set"). The bead doesn't enumerate which new mechanisms emit, where in priority order they slot, or how they interact with `prediction_confirmed` / `validator_widening` / `unknown_no_fit`.

This is left to spec drafting and is appropriate at bead-acceptance level — but if the spec inherits the bead's silence, the implementing agent will pick rule order ad-hoc. MADR 0081 (mechanism vocabulary) and MADR 0075 (rule-cascade structure) constrain the choice, but the new mechanism tokens are net-new.

**Evidence:**
- Bead ac-08: "Cascade gains rules for new signals; Phase 1 cascade rules unchanged"
- `.orbit/specs/2026-05-04-autonomous-type-inference/spec.yaml:472` — "10-element closed set"
- `.orbit/choices/0081-mechanism-vocabulary-aligned-with-madr-0075.md` — vocabulary rules

**Recommendation:** Spec must enumerate (a) the new mechanism tokens for generator-shape and sibling-context (e.g. `shape_consensus`, `sibling_context_match`?), (b) their priority order vs the existing 10 rules, and (c) any cascade interactions (e.g. does a strong shape signal short-circuit the existing `validator_widening` rule?). At bead-acceptance level, recommend a one-line addition to ac-08: "Spec enumerates new mechanism tokens with priority position and cascade interaction; Phase 1's 10-rule order is preserved (new rules slot before fallback)."

### [LOW] No reference to MADR 0084's methodology pass-2 lesson

**Category:** missing-requirement
**Pass:** 2
**Description:** MADR 0084 lines 141-150 added a methodology rule for review-spec: "when an AC carries a numeric target (`≥X% on dataset Y`), review-spec MUST require either (a) a prior measurement citing the source, or (b) an explicit 'unbacked target' annotation". The v2 bead acceptance carries multiple numeric targets (ac-04's 60%, ac-05's 0.304 / 0.014×2, ac-06's 30ms / 100ms, ac-07's ≥60% concentration). All cite a prior measurement (good — that's the methodology lesson honoured) EXCEPT ac-07's "≥60% concentration" — that number is not anchored to any prior signal-attribution evidence.

The 60% concentration threshold is plausible but un-anchored. If the actual concentration is 50%, is the AC's "predictability criterion" considered failed (and hence Phase 2's signals declared non-decisive)? If 70%, what does it mean? The number was picked without back-up.

**Evidence:**
- Bead ac-07: "≥60% by row count) in shape-driven types" / "≥60%) in neutral-header columns"
- MADR 0084:141-150 — methodology lesson
- No evidence in repo for a 60% concentration baseline

**Recommendation:** Annotate ac-07's two ≥60% thresholds as "exploratory targets — no prior signal-attribution baseline exists; Phase 2 is itself the first measurement". This is the "(b) unbacked target annotation" path of MADR 0084's lesson — it doesn't require a prior measurement, just acknowledgement that the number is unbacked. Then ac-07 is honestly framed: a positive concentration result is informative; a low concentration tells us the new signals are not as orthogonal as hoped.

---

## Honest Assessment

The v2 acceptance criteria are dramatically tighter than v1. All three v1-HIGH findings are addressed substantively: ac-05 binds the cliff-lift to a number, ac-02 (now the MADR-0085 prerequisite gate) reconciles the architectural conflict with MADR 0083, and ac-01 plus ac-04's modal-expectation framing kills the "60% as silent default" pattern. The MEDIUM and LOW findings from v1 (latency cost-model, ablation methodology, schema migration, leakage path, gate markers) are likewise addressed.

The remaining HIGH finding is the inherited numeric error (the 0.014 baseline in ac-05). Catching this is exactly the kind of empirical-anchor check MADR 0084's pass-2 methodology lesson asks reviewers to do — re-derive the number, don't just check internal consistency. The finding is HIGH because ac-05 is the falsifiable load-bearing gate and a 2× too-low target lets Phase 2 pass against a null. The fix is mechanical (recompute the trapezoid).

The second HIGH (ac-03 weight-discipline OR clause) is real but lower-stakes than the v1 architecture conflict it replaces — leakage IS firewalled in both branches; the open question is which review surface and which deliverable Phase 2 commits to. Recommend (a) by inheritance from MADR 0079.

The MEDIUM gate-placement findings (ac-04 and ac-01 ungated) are simple fixes — add `[gate]` markers — that meaningfully tighten implement-skill enforcement.

REQUEST_CHANGES rather than BLOCK because the bead's structural shape is sound, the v1 surface has been seriously addressed, and the remaining findings are bounded to numeric correction (ac-05 baseline), one binary choice (ac-03 (a) vs (b)), and gate-marker placement. Two of three are mechanical edits; the third is a design call that should take less than an hour.

APPROVE on the next pass once: (1) ac-05's 0.014 baseline is recomputed or the AUC line is demoted to non-gating, (2) ac-03 picks (a) or (b) with rationale, (3) ac-04 carries [gate], (4) ac-01 is gated or its content links into ac-03's required rationale.
