# Interview: Distillation v3 — Full Dataset Coverage

**Date:** 2026-03-19
**Interviewer:** Nightingale

---

## Context

After an incident, all distillation v2 output was lost (merged_labels.csv, batch CSVs, rescore data). We've since downloaded four benchmark datasets to ~/datasets/: GitTables 1M (56 GB, 94 topics), SOTAB V2 (7.5 GB), Sherlock/VizNet (745 MB, 687K columns with 78-type ground truth), and SportsTables (scrapers + annotations). The question is how to structure the next distillation run to maximise value from this data.

## Interview Q&A

### Q1: Goal
**Q:** What's the primary goal — restore the Phase 2 baseline for PR-2 rescore, or expand to a fresh Phase 3?
**A:** Restore Phase 2 baseline first. Need a clean before/after to prove PR-2's value.

### Q2: Scope — GitTables selection
**Q:** Phase 2 processed 507 CSVs from GitTables. The original file list is lost. Re-extract from scratch or try to reconstruct?
**A:** If we don't actually know what was done in Phase 2, we should start with the highest value data.

### Q3: Value axis
**Q:** What does "highest value" mean for ordering the distillation?
**A:** Weakness targeting — prioritise data that hits known FineType gaps (numeric disambiguation, categorical/ordinal, entity vs sentence).

### Q4: Structure
**Q:** Should we structure distillation in targeted waves per weakness, or run broad and filter after?
**A:** Hybrid: broad first, targeted second. Wave 1 broad across all datasets, analyse gaps, Wave 2 targeted at weaknesses with insufficient evidence.

### Q5: Scale
**Q:** What's the target scale for Wave 1?
**A:** Distill everything we've downloaded by end of week. Token impact from yesterday was acceptable (17% used, resets in ~26 hours). Bank the data now, analyse later.

### Q6: Dataset inclusion
**Q:** Which datasets should be in the distillation plan?
**A:** GitTables + eval CSVs + Sherlock + SOTAB. Skip SportsTables (requires scraping live websites, fragile).

### Q7: GitTables sampling
**Q:** GitTables has ~368K parquet files. What sampling strategy?
**A:** Size-based: all files under 50 columns. Smaller tables are faster to distill and tend to have cleaner column semantics.

### Q8: Sherlock handling
**Q:** Sherlock already has 78-type ground truth labels. Does it still need blind-first distillation?
**A:** Yes, still valuable to distill. Three-way comparison (Sherlock label vs Claude blind vs FineType) provides an external benchmark anchor.

### Q9: Sherlock scale
**Q:** Sherlock is 687K columns. What's the budget?
**A:** Full distillation. Prioritise Sherlock — do it first.

### Q10: Eval suite update
**Q:** How should distillation outputs feed back into the eval suite?
**A:** Open question. Current eval is 174 entries — too small for both regression protection and accuracy benchmarking. The distillation should help us design for both. No strong view on tiering yet.

### Q11: Execution order
**Q:** What's the execution order across the week?
**A:** Sherlock → SOTAB → GitTables → eval CSVs.

### Q12: Distillation protocol for Sherlock
**Q:** Should the distillation agent see Sherlock's ground truth label during adjudication?
**A:** No — leave it out. The Sherlock label is fixed, can't be amended. Use blind → FineType adjudication (same as Phase 2). Compare against Sherlock labels offline to find taxonomy coverage gaps.

---

## Summary

### Goal
Distill all downloaded datasets (Sherlock, SOTAB, GitTables, eval CSVs) using the Phase 2 blind-first adjudication protocol. Bank comprehensive disagreement data this week while token budget allows. Use results to: (1) establish a new baseline for PR-2 rescore, (2) identify FineType weaknesses across diverse data, (3) check taxonomy coverage against Sherlock's 78-type ground truth.

### Constraints
- Claude Max 20x plan — 17% used this week, resets in ~26 hours
- Beelink mini PC (16GB RAM, 5 agents max)
- Same blind-first adjudication protocol as Phase 2
- Sherlock ground truth compared offline, not during distillation
- SportsTables excluded (requires live scraping)
- GitTables sampled: files under 50 columns only

### Execution Order
1. **Sherlock** (687K columns, 78-type ground truth) — highest priority
2. **SOTAB** (JSON extraction step needed, then distillation)
3. **GitTables** (extract CSVs from parquet, files under 50 columns, across all 94 topics)
4. **Eval CSVs** (27 existing files, small, slot in at the end)

### Outputs
- `output/distillation-v3/` — per-batch CSVs + merged_labels.csv
- Sherlock→FineType type mapping (78→250 types, programmatic)
- Three-way comparison report: Sherlock ground truth vs Claude vs FineType
- Disagreement analysis by weakness category (maps to PR-2 fixes)
- Eval suite expansion recommendations

### Success Criteria
- All four datasets distilled by end of week
- Merged labels file with blind_label, finetype_label, final_label, reasoning
- Per-weakness disagreement counts comparable to Phase 2 findings
- Sherlock coverage gap analysis identifies any FineType taxonomy holes
- Data sufficient to rescore PR-2 fixes

### Open Questions
- How to structure the eval suite after distillation (gold/silver tiers vs expanded manifest)
- SOTAB extraction step: use existing column_values.parquet or re-extract from JSON?
- Exact GitTables file count after 50-column filter (need to scan parquet metadata)
- Whether to map Sherlock's 78 types to FineType taxonomy before or after distillation
- Token budget allocation across Sherlock (687K cols) vs the other datasets
