# Design: Sharpen Rule Audit + v19 Promotion

**Date:** 2026-04-27
**Interviewer:** Nightingale
**Card:** .orbit/cards/0002-semantic-type-detection.yaml
**Mode:** design (full auto, grounded in interactive discovery session)

---

## Context

Card: *Semantic type detection* — 4 scenarios, goal: 215+/227 (~95%).
Prior specs: 13 shipped — most recent: v19 paired retrain (MADR 0066 gate FAIL,
v16 remains at 371/448 on expanded eval). Amount-variant fix (+11 via pipeline
layer, MADR 0065). Sharpen demotion guard (MADR 0059). Header hint removal
(MADR 0042). Value rules only (MADR 0048). Strength through simplification
(MADR 0038).

Gap: v19 model has better val_acc (91.3% vs ~91%) but Sharpen rules fight it
on 16 columns → net −6 profile eval. Removing destructive rules unblocks the
improved model.

## Discovery Q&A (interactive, author-answered)

### Q1: Ship strategy
**Q:** Rules + model together in one PR, or rules first then model later?
**A:** Rules + model together. One PR, one eval cycle.

### Q2: Eval scope
**Q:** Should coverage_closure (synthetic) count toward the MADR 0066 gate?
**A:** Include everything. Full 448-row manifest, no special cases.

### Q3: Fallback
**Q:** If rule removal + v19 still doesn't pass MADR 0066 gate?
**A:** Lower the gate. Accept a tie (net_label_delta = 0) as sufficient given
better val_acc + fewer rules.

### Q4: Scope
**Q:** Audit only the 16 regression-implicated rules, or full Sharpen audit?
**A:** Full Sharpen audit of every F and R rule for net impact.

## Design Q&A (full-auto, self-answered from card + evidence)

### Q5: Outcome priority — accuracy vs rule count
**Q:** When a rule is net-positive on v16 but net-zero on v19, do we keep it
(preserving v16 fallback) or remove it (cleaner pipeline for future retrains)?
**A:** Remove it. Per decision 0038 (strength through simplification), prefer a
model that can stand on its own. v16 fallback is not a design goal — we're
shipping v19. Rules that only help v16 are technical debt if v19 doesn't need them.

### Q6: Risk appetite — what breakage is tolerable?
**Q:** How much v16 regression is acceptable as collateral from rule removal?
**A:** Not applicable — we are not shipping v16. The gate evaluates v19-relu +
cleaned pipeline against the v16 baseline. v16's score is the floor to beat (or
tie), not a thing to preserve. Per card scenario "Column-level disambiguation":
the model should do the disambiguation, not rules.

### Q7: Header hints — in scope or out?
**Q:** Header hints (semantic.rs) are the known destructive mechanism from
amount-variant (MADR 0065). Include in the audit alongside F/R rules?
**A:** Yes. Header hints are Sharpen post-processing. The full audit means all
three subsystems: feature_sharpen (F rules), value_sharpen (R rules), and
apply_header_sharpen (semantic hints). Decision 0042 already deprecated regex
hints; this audit measures the remaining semantic hints' net impact.

### Q8: Gate amendment — formal decision needed?
**Q:** Does lowering MADR 0066 to accept a tie need a formal MADR amendment?
**A:** Yes. The gate is referenced across multiple specs and drives. An amendment
to MADR 0066 (or a new decision superseding the relevant clause) makes the
change explicit and auditable. Record as part of this drive.

---

## Summary

### Goal
Full audit of all Sharpen post-processing rules (F1–F6, R1–R31, header hints).
Remove net-negative rules. Ship rule removals alongside v19-relu-s42 promotion.
Amend MADR 0066 gate to accept a tie (net_label_delta ≥ 0 with justification).

### Constraints
- Full 448-row manifest for gate evaluation
- Rules + model ship together (one PR)
- MADR 0066 gate, amended to accept tie (≥ 0) when rule count decreases
- No Sharpen additions — only removals and narrowing
- All three Sharpen subsystems in scope (feature, value, header)

### Success Criteria
- Every Sharpen rule has a measured net impact on the 448-row manifest
- Net-negative rules removed
- v19-relu + cleaned pipeline ≥ v16 baseline (371/448) on label match
- Rule count decreases
- MADR amendment recorded

### Decisions Surfaced
- **Rules + model together:** one PR, one eval cycle (author Q1)
- **Accept a tie at the gate:** net_label_delta ≥ 0 acceptable when rule count
  decreases (author Q3, → amend MADR 0066)
- **Full audit:** all F, R, and header hint rules (author Q4, design Q7)
- **Remove v16-only rules:** rules that only help v16 are technical debt (design Q5,
  grounded in decision 0038)

### Implementation Notes
- Diagnostic method: ablation study — disable each rule individually, score v19-relu
  against full manifest. DuckDB scoring from v19_compare.sh is ready.
- The 16 regressions cluster: 7 coverage_closure, 3 datetime subtype, 3 cross-domain,
  3 scientific/text. Header hints are the likely mechanism for cross-domain jumps.
- F rules live in `feature_sharpen()` (column.rs), R rules in `value_sharpen()`
  (column.rs), header hints in `apply_header_sharpen()` (column.rs) calling
  `semantic.rs`.
- Sharpen demotion guard (decision 0059) is a separate mechanism — audit but
  likely net-positive (it prevents rule-based demotions when validator confirms).
- v19_compare.sh scoring infrastructure is working — reuse for ablation measurements.

### Open Questions
- None at intent level. Implementation questions (which rules, what mechanisms)
  are traced during the diagnostic phase.
