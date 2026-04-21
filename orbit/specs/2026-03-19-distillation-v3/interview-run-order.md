# Interview: Distillation v3 — Run Order & Eval Strategy

**Date:** 2026-03-19
**Interviewer:** Nightingale

---

## Context

Distillation v3 is 48 Sherlock batches in (4,800 columns, 3.5% of Sherlock test split). 27.5% agreement rate on headerless data. All four source JSONLs are extracted and ready (Sherlock 137K, SOTAB 361K, GitTables 45K, eval 282 columns). Question: what order to process, and how findings feed back into the eval suite.

## Interview Q&A

### Q1: Sherlock depth
**Q:** We've burned ~4,800 Sherlock columns (headerless, 27.5% agreement). Sherlock has 132K columns left but they're all headerless with the same 78 semantic types — diminishing returns on new signal. Should we finish a meaningful Sherlock sample and then pivot to headed data, or keep grinding Sherlock?
**A:** Finish all 137K. Complete the test split before moving on.

### Q2: Next dataset after Sherlock
**Q:** After Sherlock, what's the highest-value next dataset?
**A:** Eval CSVs (282 cols). These ARE the eval suite — distilling them gives direct before/after comparison when fixes land. Done in 3 batches (~15 min).

### Q3: Eval suite update strategy
**Q:** How should distillation findings flow back into the eval suite? The current eval has 174 columns across 30 datasets — distillation is producing orders of magnitude more labelled data.
**A:** Didn't have a strong opinion — asked for recommendation.

**Nightingale's recommendation (accepted):** Two-tier approach:
- **Tier 1 — Keep existing eval as-is** (174 cols, 31 datasets). Stable regression baseline. Hand-curated, human-reviewed, known baseline (170/174 = 97.7%). Answers "did I break anything?"
- **Tier 2 — Build stratified distillation benchmark** (~500-1000 cols). Algorithmically sampled from distillation output: N columns per FineType type, balanced by agreement/disagreement, mixed headerless (Sherlock) + headed (SOTAB/GitTables), external ground truth where available. Answers "how does FineType perform across the full taxonomy and diverse data?" New `eval/distillation_benchmark/` with its own manifest.

### Q4: Third dataset (after Eval CSVs)
**Q:** GitTables (45K, 94 topics, headed) or SOTAB (361K, Schema.org ground truth, headed)?
**A:** SOTAB next. Schema.org ground truth enables three-way comparison.

---

## Summary

### Execution Order
1. **Sherlock** — finish all 137K columns (1,374 batches, ~1,326 remaining)
2. **Eval CSVs** — 282 columns (3 batches, ~15 min)
3. **SOTAB** — 361K columns (3,611 batches)
4. **GitTables** — 45K columns (448 batches, if token budget allows)

### Eval Strategy
Two-tier:
- Tier 1: Existing eval (regression, untouched)
- Tier 2: Distillation benchmark (stratified sample, built after all datasets complete)

### Rationale
- Sherlock first: complete the standard benchmark partition for full per-type error rates
- Eval CSVs second: tiny, gives immediate regression comparison data
- SOTAB third: Schema.org ground truth is uniquely valuable for three-way comparison
- GitTables last: useful but no external ground truth, lower priority

### Open Questions
- Exact stratified sampling algorithm for Tier 2 benchmark (deferred until distillation complete)
- Whether Tier 2 needs human review pass or can be fully algorithmic
- SOTAB Schema.org → FineType type mapping (needed for three-way comparison)
