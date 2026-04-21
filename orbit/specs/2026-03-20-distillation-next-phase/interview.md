# Interview: Distillation Next Phase — Benchmark & Retraining

**Date:** 2026-03-20
**Interviewer:** Nightingale

---

## Context

PR #16 merged — 85,194 distilled Sherlock columns landed as `sherlock_distilled.csv.gz`. Pipeline is paused but will resume in the background over the coming week. Decision 0038 superseded the v2 heuristic fixes in favour of a retraining path.

Current state:
- 172/250 taxonomy types represented in distilled data
- 78 types missing — mostly format-specific (timestamps, financial identifiers, containers, scientific formats)
- 111 types have zero real-world coverage (only synthetic generators)
- Top types heavily skewed: categorical (15,363), entity_name (15,070), plain_text (8,366)
- Profile eval baseline: 170/174 (97.7%)
- Pipeline remaining: 522 Sherlock + 3 Eval + 448 GitTables + 3,611 SOTAB

The missing types are structural — Sherlock alone won't fill them regardless of batch count. The gaps are in datetime (40 missing), finance (21), identity (16), technology (15), geography (7), container (6), representation (6). These types need synthetic/generated data as their training source.

## Interview Q&A

### Q1: Data threshold
**Q:** What's the minimum data threshold before we start building the Tier 2 benchmark and retraining spike?
**A:** Current data is enough to start. The distilled real-world data strengthens common types; synthetic/generated data covers the 78 missing types. Both sources are needed — don't wait for full pipeline completion.

### Q2: Benchmark scope
**Q:** Should the Tier 2 benchmark cover all 250 types (using synthetic for gaps), or only the 172 types where we have real-world evidence?
**A:** All 250 types. Full taxonomy coverage.

### Q3: Spike structure
**Q:** The retraining spike has two goals: improve accuracy via real-world data, and absorb disambiguation rules into the model. Run together or sequentially?
**A:** Accuracy first. Spike 1: retrain with blended data, measure Tier 2 accuracy. Spike 2: attempt rule removal one-by-one against the retrained model. Sequential — build confidence before removing rules.

### Q4: Pipeline priority
**Q:** What's the pipeline priority and budget appetite for completing the remaining sources?
**A:** Run the spike with the data we have. Pipeline will resume in the background and take around a week. Don't block on it.

### Q5: Tier 2 benchmark design
**Q:** How many columns per type, and does it need human review?
**A:** 10 per type, algorithmic (~2,500 columns total). For distilled types: sample from agreement rows (where blind agent and FineType both agreed). For the 78 missing types: use generator output. No human review — can be rebuilt automatically when more data arrives.

### Q6: Sequencing
**Q:** What's the ordering priority for the three specs relative to the background pipeline?
**A:** Tier 2 now, spike when ready. Build the benchmark immediately from current 85K data. Start retraining spike as soon as Tier 2 is built. Pipeline runs in parallel — don't wait for it to finish.

---

## Summary

### Goal
Build a Tier 2 benchmark (2,500 columns, all 250 types) from distilled + synthetic data, then run a sequential retraining spike: first improve accuracy with blended training data, then attempt disambiguation rule removal.

### Constraints
- Profile eval (170/174, 97.7%) is the regression floor
- Pipeline continues in background (~1 week) — don't block on it
- Tier 2 is algorithmic (no human review), rebuilt when more data arrives
- Retraining spike is investigative — no commitment to ship a retrained model
- Rule removal is a separate spike, only after accuracy spike succeeds
- Current distilled data (85K rows, 172 types) is the starting point; 78 types use synthetic generators

### Success Criteria
- Tier 2 benchmark built: ~2,500 columns, 250 types, reproducible build script
- Retraining spike completed: synthetic vs distilled vs blended accuracy on Tier 2
- Profile eval (Tier 1) does not regress on retrained model
- Clear signal on whether blended training improves common-type accuracy
- Candidate list of disambiguation rules that a retrained model could absorb

### Open Questions
- Exact sampling strategy for agreement rows (random? stratified by confidence?)
- How many synthetic columns per missing type if generator output is small
- Which model architecture changes (if any) the retraining spike should explore
- Whether Tier 2 results should gate the rule-removal spike or just inform it

### Sequencing
```
Spec 1: Tier 2 Benchmark Build ─────────────────► immediate
Spec 2: Retraining Spike (accuracy) ────────────► after Tier 2
Spec 3: Rule Removal Spike ─────────────────────► after Spike 2
Background: Pipeline completion (Sherlock → Eval → GitTables → SOTAB) ──► parallel
```
