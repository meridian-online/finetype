# Interview: Distillation v2 Fixes

**Date:** 2026-03-18
**Interviewer:** Nightingale

---

## Context

Phase 2 distillation completed: 5,364 columns from 507 CSV files classified via blind-first adjudication using Claude Sonnet agents. 36% agreement with FineType, with 10 systematic gap categories identified and ranked by impact. Hugh wants to implement fixes before running Phase 3 (wild data expansion) so that Phase 3 validates an improved engine.

Findings: `specs/2026-03-18-distillation-v2/FINDINGS.md`

## Interview Q&A

### Q1: Fix scope
**Q:** The findings rank 10 fixes by impact. What's your appetite for fix scope before Phase 3 — do you want to tackle just the quick wins (low difficulty), or go deeper?
**A:** All low + medium fixes (1-9). That's ~880 estimated disagreement flips across: boolean binary heuristic, categorical vs ordinal default, "points" header hint guard, username demotion guard, float-stored ID detection, epoch seconds range check, sequential ID detection, sentence vs entity_name length heuristic, and financial header hint domain guards.

### Q2: PR strategy
**Q:** These fixes touch different parts of the pipeline: header hints (1, 3, 6, 9), column-level heuristics (2, 4, 5, 7, 8), and value classification. Should we implement them as one PR or batch them into logical groups?
**A:** Grouped by subsystem. ~3 PRs: header hint fixes, column heuristic fixes, then a combined eval pass. Balanced traceability without 9 separate review cycles.

### Q3: Eval strategy
**Q:** How do we measure success for each PR group? The distillation labels are our new reference, but we also have the existing profile eval (170/174). Which matters more?
**A:** Both must improve. Profile eval must not regress AND distillation agreement must increase. But if a fix is correct by distillation evidence and regresses profile eval, update the eval expectations — the eval serves the engine, not the other way around. (Recorded as Decision 0037.)

### Q4: Comparison method
**Q:** Re-running the full distillation pipeline after each PR group would cost ~4 hours of Claude compute. How do you want to measure distillation agreement improvement?
**A:** Entirely offline. We already have Claude's blind labels for all 5,364 columns. Re-run `finetype profile` on the same files, join against `merged_labels.csv`, and count agreements vs the Phase 2 baseline of 1,983/5,364. No new Claude calls needed.

### Q5: Eval cadence
**Q:** Should we run the offline re-score after each PR group or just once at the end?
**A:** After each PR group. 3 re-scores total: after header hint fixes, after column heuristic fixes, and final combined. Shows incremental attribution and catches surprising interactions.

### Q6: Default behaviour
**Q:** Fix 2 (categorical vs ordinal) and fix 4 (username demotion guard) change column-level disambiguation defaults. Are you comfortable changing defaults directly, or do you want new logic behind a flag?
**A:** Change defaults directly. Also, if the eval is wrong — or it's holding us to a bad architecture, it's OK to change.

### Q7: Header hint strategy
**Q:** The header hint fixes (1, 3, 6, 9) all point to the same root cause: substring matching is too greedy. The findings suggest two approaches — regex value-pattern guards, or migrating hint patterns into the taxonomy YAML. Which direction?
**A:** Taxonomy YAML migration. Move header hint patterns into `labels/definitions_*.yaml` with contextual constraints. Aligns with the taxonomy-as-source-of-truth principle.

### Q8: Sequencing with Phase 3
**Q:** Given Phase 3 is planned for tomorrow, how do you want to sequence?
**A:** Fixes first, Phase 3 waits. Ship all 9 fixes before starting Phase 3. Phase 3 then validates the improved engine against wild data — gives the most useful signal.

---

## Summary

### Goal
Implement all 9 low-to-medium difficulty fixes identified by the distillation v2 findings, improving FineType's agreement rate from 36% toward an estimated ~52% before Phase 3 wild data expansion begins.

### Constraints
- Grouped into ~3 PRs by subsystem (header hints, column heuristics, combined eval)
- Header hint fixes via taxonomy YAML migration (not code-level regex guards)
- Default behaviour changes directly (no feature flags)
- Offline re-score after each PR group using banked distillation labels
- Phase 3 waits until all fixes land

### Success Criteria
- Profile eval (170/174) must not regress, or regressions are justified by distillation evidence (Decision 0037)
- Distillation agreement increases from 1,983/5,364 baseline after each PR group
- All 9 fixes from the priority table shipped and evaluated

### PR Groups

**PR Group 1 — Header hint fixes (taxonomy YAML migration):**
- Fix 3: "points" header hint guard (+114)
- Fix 6: Epoch seconds range check (+79) — "created" hint
- Fix 9: Financial header hint domain guards (+66) — "yield", "pct", "rate"
- Fix 1: Boolean binary heuristic (+175) — partially header-triggered

**PR Group 2 — Column heuristic fixes:**
- Fix 2: Categorical vs ordinal default (+123)
- Fix 4: Username demotion guard (+92)
- Fix 5: Float-stored integer ID detection (+83)
- Fix 7: Sequential ID detection (+77)
- Fix 8: Sentence vs entity_name length heuristic (+71)

**PR Group 3 — Eval update + combined re-score:**
- Update profile eval expectations if needed
- Final combined distillation re-score
- Document before/after metrics

### Decisions Made
- **0037** — Eval serves the engine: distillation evidence can override profile eval expectations

### Open Questions
- Exact taxonomy YAML schema for header hint patterns (needs design in the spec)
- Whether fix 1 (boolean binary) belongs in PR group 1 (header-related) or group 2 (column heuristic) — it's partially both
- How to structure the offline re-scoring script (one-off or reusable for Phase 3)
