# Design: v13 Retrain

**Date:** 2026-04-16
**Interviewer:** Nightingale
**Input:** `specs/2026-04-16-v12-data-quality-audit/retrain-brief.md`

---

## Context

The v12 data quality audit identified 23 misclassifications with root causes: model_error (9), training_collision (8), data_gap (6). The retrain brief proposes 4 priority tiers of fixes. This design session scoped the v13 training run.

## Q&A

### Q1: Scope — which tiers ship in v13?
**Q:** P1+P2 only (safe wins), P1+P2+P3, or all four tiers?
**A:** All four. Ship everything in one retrain.

### Q2: P1 state_code remap — drop or create new type?
**Q:** The `data/label_remap.json` maps `state_code → country_code`, contaminating country_code training with US state abbreviations. Drop the rows, or create a `state_code` taxonomy type?
**A:** Create `state_code` as a new taxonomy type. It's a valid type that occurs frequently in real-world analysis.

### Q3: P3 distilled cap — hard cap or proportional?
**Q:** Cap distilled rows at 600/type (simple) or use a proportional cap (median-based)?
**A:** Hard cap at 600. Simple and well-reasoned.

### Q4: P4 architecture change
**Q:** Comfortable with validation branch resize (239→128→64 to 239→192→128, merged dim 1006→1070)? Where should gradient norms go?
**A:** Yes, comfortable with the architecture change. Nightingale's call on gradient norms.
**Decision:** Gradient norms to `results.json` — structured, per-branch, per-epoch.

### Q5: P2 latitude validation — add or skip?
**Q:** Latitude validation (`[-90, 90]` range) matches most small decimals. Skip for model signal and rely on hard-negative mining instead?
**A:** Add it. These aren't mutually exclusive. Range validation serves the schema validation pipeline (`finetype validate`), not just the model.

### Q6: Training configuration
**Q:** Same 1,200 columns/type and 70/30 blend? Epochs?
**A:** Be ambitious — hit the sprint goal. Corrected that v12 ran 40 epochs (not 30), peaked at epoch 38.
**Decision:** 50 epochs for v13. Same 1,200 columns/type, same blend ratio. Overnight run with auto-eval.

---

## Summary

### Goal
Retrain multi-branch model (v13) with improved training data, expanded validation patterns, rebalanced class distribution, and increased validation branch capacity. Target: ≥210/227 (92.5%+).

### Constraints
- All 4 priority tiers ship together
- New `state_code` taxonomy type (not just dropping rows)
- Hard cap distilled at 600/type
- Architecture change: validation branch 239→192→128
- 50 epochs, overnight run

### Decisions Surfaced
1. All four tiers in v13 — no phased rollout
2. Create `state_code` taxonomy type — not just drop contaminated rows
3. Hard cap 600 distilled/type — simple, not proportional
4. Validation branch resize accepted — merged dim 1006→1070
5. Gradient norms to `results.json` — structured metrics
6. Add latitude range validation — serves schema validation pipeline too
7. 50 epochs — up from v12's 40, which hadn't fully converged

### Open Questions
- None
