# Spec Review (v3)

**Date:** 2026-05-04
**Reviewer:** Context-separated agent (fresh session)
**Bead:** finetype-dnf
**Cycle:** 3 (drive_review_spec_cycle = 2 in metadata; this is the third review pass)
**Verdict:** APPROVE | REQUEST_CHANGES | BLOCK

**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 0 |
| 2 — Assumption & failure | content signals (full-partition measurement boundary, gate-marker pattern, mechanism-cascade slot ordering) | 1 |
| 3 — Adversarial | not triggered (Pass-1 zero, Pass-2 one LOW) | 0 |

## Diff vs v2 review

The bead's `acceptance_criteria` field was updated 2026-05-04T10:32:40Z, after the v2 review (file mtime 20:30 local on the same day). All seven v2 findings have been addressed substantively. Mapping:

| v2 finding | v3 disposition |
|---|---|
| HIGH — ac-05 baseline AUC value 0.014 numerically incorrect | RESOLVED: AUC demoted to non-gating sidecar diagnostic per v2 recommendation (2). The gating criterion is now the unambiguous "non_unknown@0.5 ≥ 0.304" (≥10 pp absolute over 0.204). AUC remains in the sidecar with a recomputed Phase-1 baseline on the same partition called out explicitly. |
| HIGH — ac-03 weight-discipline OR clause | RESOLVED: ac-03 picks branch (a) — design-time MADR lock mirroring MADR 0079 — and explicitly forbids the implementing agent from running a sweep against either calibrate or measure half. Rationale must cite (a) first-principles reasoning AND (b) per-signal lift estimates from ac-01's memo. |
| MEDIUM — ac-04 not gated | RESOLVED: ac-04 carries `[gate]`. |
| MEDIUM — ac-01 plausibility memo not gated | RESOLVED: ac-01 carries `[gate]` AND adds the soft-prerequisite content link ("per-signal lift estimates MUST be cited verbatim in ac-03's weight rationale and ac-04's outcome framing"). The v2 recommendation listed (1) and (2) as alternatives; v3 took both, which is stronger than either alone. |
| MEDIUM — ac-06 mitigation trigger conditional, estimate not audited | RESOLVED: ac-06 now requires three concrete cost-model components — (i) K pinned to a specific integer; (ii) per-generator µs estimate from a real measurement with a worked example; (iii) mitigation triggered up front when (estimated cost) × 1.5 > 30ms remaining headroom. The 1.5× safety margin guards the under-estimation failure mode v2 flagged. |
| LOW — ac-08 cascade rule additions not enumerated | RESOLVED: ac-08 now requires the spec to enumerate new mechanism tokens (candidates `shape_consensus`, `sibling_context_match`), priority position, interaction with Phase 1's existing 10 rules, and the default rule-slot heuristic ("new rules slot before the fallback rules unless spec rationale argues otherwise"). MADR 0081 update is named additively. |
| LOW — ac-07 ≥60% concentration unbacked | RESOLVED: ac-07 explicitly annotates the two ≥60% thresholds as "EXPLORATORY TARGETS per MADR 0084 methodology lesson '(b) unbacked target annotation'" with "informative either way, no auto-fail". This is the methodology lesson honoured verbatim. |

The v3 acceptance criteria are tight, internally consistent, sequenced correctly via gates, and anchored to measurements where they make claims. Pass 1 surfaced no structural issues. Pass 2 surfaced one LOW finding (recorded below) which is not a blocker.

## Findings

### [LOW] ac-05 gate compares Phase 2 full-partition number to Phase 1 1000-row sample baseline — minor apples-to-oranges, well-bounded

**Category:** test-gap
**Pass:** 2
**Description:** ac-05's gating criterion is "non_unknown rate at threshold 0.5 on the measure half (full 10,660 rows) rises by ≥10 pp absolute over Phase 1's 0.204 baseline". Phase 2 is required to compute its own number on the full 10,660 rows, but the 0.204 baseline came from a 1000-row deterministic sample (per progress.md:159-160 — "Per-threshold curve (calibrate vs measure, deterministic sample of 1000 rows each)"). The AC's parenthetical names this exactly ("Phase 1's progress.md numbers were computed on a 1000-row deterministic sample, not the full partition") but only addresses it inside the non-gating sidecar diagnostic — the gating criterion itself still references "0.204" as if it were a full-partition number.

The numerical risk is bounded. progress.md:174 records "calibrate vs measure differ by ≤0.5 pp at every threshold" and the headline @0.7 rate (0.071) matches the full-partition 7.1% to 0.001, so the 0.5-threshold rate on the full partition is almost certainly within ±1 pp of 0.204. A 10 pp absolute lift target is robust to ±1 pp baseline drift.

But the gating language could be tightened. As written, an implementing agent could meet 0.304 via Phase 2's full-partition measurement against a baseline (0.204) computed differently. If the recomputed Phase-1 baseline on the full partition is 0.211 (within the ±0.5 pp tolerance), the implicit lift target is 0.311, not 0.304. The discrepancy doesn't matter for a clean 0.4 result; it matters at the margin.

**Evidence:**
- Bead ac-05 [gate]: "non_unknown rate at threshold 0.5 on the measure half (full 10,660 rows) rises by ≥10 pp absolute over Phase 1's 0.204 baseline (i.e. ≥0.304 measured)"
- Bead ac-05 sidecar clause: "alongside a recomputed Phase 1 baseline on the same partition (Phase 1's progress.md numbers were computed on a 1000-row deterministic sample, not the full partition)"
- `.orbit/specs/2026-05-04-autonomous-type-inference/progress.md:159-174` — the 1000-row sample table and ≤0.5 pp calibrate-vs-measure agreement statement
- `.orbit/choices/0084-ac02-floor-empirical-recalibration.md:141-150` — methodology lesson on numeric anchors

**Recommendation:** Either of the following resolves the ambiguity at the AC level (mechanical edit, not a design call):

1. Change ac-05's gating clause to "rises by ≥10 pp absolute over the recomputed Phase 1 baseline on the same partition (target: ≥0.304 measured if recomputed baseline matches the 1000-row sample's 0.204; otherwise: recomputed_baseline + 0.10)". This binds the lift target to the recomputed full-partition number that ac-05's own sidecar already requires.

2. Keep the literal 0.304 target but add: "Phase 2 must first recompute Phase 1's full-partition non_unknown rate at threshold 0.5; the AC passes if the Phase 2 rate satisfies BOTH (a) ≥0.304 absolute AND (b) ≥10 pp over the recomputed Phase 1 baseline."

(1) is simpler. (2) is more conservative. Either is acceptable; (1) is recommended.

This is LOW because the bounding (≤0.5 pp partition variance per existing data) keeps the practical impact under 1 pp on the lift target, and the AC is internally falsifiable — it's just slightly imprecise about which baseline number it's comparing against. An implementing agent reading this AC carefully will recompute the baseline on the full partition (the sidecar requirement forces it) and use the larger of the two thresholds.

---

## Honest Assessment

The v3 acceptance criteria represent a substantive tightening over v2. Every v2 finding was addressed, and the resolutions are correct in shape:
- The numeric-anchor failure mode that MADR 0084 was specifically authored to catch (ac-05's 0.014 baseline) is now resolved by demoting AUC to a sidecar — the cleanest fix.
- The weight-discipline ambiguity is resolved by inheritance from MADR 0079 (branch a), with an explicit ban on agent-driven sweeping.
- Gates are now correctly placed on the four ACs that block bead-close: ac-01 (memo as design input), ac-02 (MADR 0085 prerequisite), ac-04 (narrative deliverable), ac-05 (falsifiable claim), ac-06 (latency budget).
- ac-07's ≥60% concentration is correctly annotated as an exploratory target per MADR 0084's methodology lesson — the spec process now visibly self-corrects against the failure mode it learnt about.

The single remaining LOW finding (ac-05's gating clause references the 1000-row-sample baseline as if it were a full-partition number) is mechanically correctable and bounded by ±1 pp on the lift target — well within the acceptable margin for a 10 pp absolute lift requirement. It does not warrant another review cycle.

APPROVE on the basis that:
1. All v2 HIGH/MEDIUM findings are resolved with the recommended approaches.
2. The bead's structural shape (Phase 2 as empirical decision over architectural extension) is sound and unchanged from v1.
3. The remaining LOW finding is a tightening opportunity for the spec drafter, not a blocker for spec-from-bead drafting.
4. The methodology lessons from MADR 0084 (numeric-anchor discipline, unbacked-target annotation, sequencing via gates) are now visibly honoured throughout the AC text.

The implementing agent should proceed with spec drafting, with the LOW finding noted as an inline tightening for the spec author to apply when the AC text is transcribed into the spec's exit_conditions / verification block.
