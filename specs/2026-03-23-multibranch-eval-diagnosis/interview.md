# Interview: Multi-Branch Eval Diagnosis & Generalization Gap

**Date:** 2026-03-23
**Interviewer:** Nightingale

---

## Context

Overnight v2 training pipeline completed (4h 51m on M1 Pro). Flat model reached
94.0% validation accuracy but only 56.8% on profile eval (vs CharCNN's 93.7%).
Hierarchical eval returned 0/0 (broken). The overnight script's grep-based result
summary also failed on macOS (BSD grep lacks `-oP`).

## Interview Q&A

### Q1: Integration Strategy
**Q:** The 37-point gap has two possible root causes: the model runs without post-processing,
or the training data distribution doesn't match real-world data. Is the goal to make
multi-branch replace the full pipeline, or slot it in as a better column classifier?
**A:** Slot into pipeline. Multi-branch replaces CharCNN as the column classifier within
Sense→Sharpen. Header hints, disambiguation rules, vote aggregation still apply.

### Q2: Integration Point
**Q:** Where should the multi-branch model plug into the pipeline?
**A:** Nightingale recommended parallel path — add `--model-type multi-branch` flag that
switches to the new column classification path. Keeps production code untouched. The
multi-branch model operates at column level (not value level), so it doesn't fit into
the CharCNN vote step architecturally. Post-processing layers can be applied on top.

### Q3: Hierarchical Eval
**Q:** The hier eval returned 0/0. Is this an eval harness bug or a model problem?
**A:** Eval harness bug. The model trained fine (93.9% val accuracy). The eval harness
doesn't know how to load/run hierarchical multi-branch.

### Q4: Eval Output Format
**Q:** The grep-based summary failed on macOS. Fix narrowly or refactor eval output?
**A:** DuckDB all the way. Eval results go into a DuckDB-queryable format. Overnight
script, CI, dashboards all query it. Markdown report generated from queries.

### Q5: Eval Storage
**Q:** Persistent database or per-run JSON with DuckDB queries?
**A:** Per-run JSON + DuckDB queries. Each eval run writes JSON. DuckDB queries the
JSON files ad-hoc. No persistent DB to manage.

### Q6: Gap Diagnosis Strategy
**Q:** Which contributes more to the 37-point gap — missing post-processing or training
data distribution? Where to invest first?
**A:** Diagnose before deciding. Run multi-branch through the pipeline WITHOUT retraining
to measure how much post-processing alone recovers. That tells us where the gap is.

### Q7: Diagnostic Approach
**Q:** Full pipeline integration or manual trace of misclassifications?
**A:** Manual trace first. Trace 10 worst misclassifications through the pipeline logic
manually. Quick, tells us which layers would help. Then decide on full integration.

### Q8: Merge Criteria
**Q:** What's the success threshold for merging PR #21?
**A:** Ship with diagnosis findings. Merge when we have: (1) working eval for both
flat+hier, (2) manual trace showing which pipeline layers close the gap, (3) a clear
plan for achieving accuracy. Accuracy target is next PR.

---

## Summary

### Goal
Fix the eval harness (hier 0/0), diagnose the 37-point generalization gap via manual
trace of misclassifications, and ship PR #21 with working eval + diagnosis findings +
a plan for closing the accuracy gap.

### Constraints
- Multi-branch slots INTO the existing Sense→Sharpen pipeline, doesn't replace it
- Parallel path integration (`--model-type multi-branch`) — don't touch production code
- Eval output refactored to per-run JSON queryable by DuckDB (no more grep/sed on markdown)
- Manual trace before full integration — measure before optimise
- macOS compatibility (no BSD grep `-oP` assumptions)

### Success Criteria
- Hierarchical eval produces real accuracy numbers (not 0/0)
- Manual trace of 10 worst misclassifications documents which pipeline layers would catch each
- Overnight script summary works on macOS (DuckDB JSON output)
- Clear plan for pipeline integration and accuracy target

### Open Questions
- None — all clarified during interview
