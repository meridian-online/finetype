# Interview: Distillation Interim — Data Landing & Improvement Path

**Date:** 2026-03-20
**Interviewer:** Nightingale

---

## Context

Distillation v3 is 60.6% through Sherlock (833/1,374 batches, 83,300 columns). Pipeline paused to manage API usage — will continue in background over the coming week. Need to land an interim stage of the work and plan what improvements it enables.

Earlier recommendations for PR preparation:
1. Concatenate into one CSV per source
2. Gzip those
3. Delete individual batch CSVs (keep .done markers)
4. Squash into one commit

Data quality review found: 27.7% agreement rate on headerless data, 161/250 types represented, 1 bad batch (0055 — column shift, flagged for re-run), categorical over-representation at 18.4%.

## Interview Q&A

### Q1: Priority
**Q:** The 83K distilled Sherlock columns give us two distinct improvement paths — fix known weaknesses with rules, or expand training data. Which is higher priority?
**A:** Both — interim PR then improvements. Land the concatenated data first, then plan both paths.

### Q2: PR scope
**Q:** For the interim PR, should we include just the concatenated distillation data, or also analysis/eval integration?
**A:** Data only. Concatenated gzipped CSV per source + pipeline scripts. Clean, minimal PR.

### Q3: Improvement approach
**Q:** FineType's biggest error patterns are IATA/ICAO false positives, country_code for states, categorical over-prediction. Which improvement approach?
**A:** Goal is to actually reduce the number of rules if possible — strength through simplification. Not more rules, better model.

### Q4: Training data mix
**Q:** How should synthetic generator data and real-world distilled data mix for retraining?
**A:** Need to investigate first. Run a spike: train models on distilled-only vs blended vs synthetic-only, compare eval scores. Let the data decide.

### Q5: Success criterion
**Q:** What would tell you the retrained model is better?
**A:** New distillation benchmark (Tier 2). Build the stratified benchmark from distillation output, then measure against that.

---

## Summary

### Goal
Land distillation v3 interim data as a clean PR, then use it to build a Tier 2 benchmark and run a retraining spike aimed at simplifying disambiguation rules.

### Constraints
- Pipeline continues in background (Sherlock → Eval → SOTAB → GitTables)
- Profile eval (170/174, 97.7%) is the regression floor — must not degrade
- Interim PR is data-only — no analysis, no eval integration yet
- Retraining spike is investigative — no commitment to a specific training mix

### Success Criteria
- Interim PR landed with concatenated, gzipped distillation data
- Tier 2 benchmark built (stratified sample from distilled data)
- Retraining spike completed with comparative results (synthetic vs distilled vs blended)
- Path to removing F1-F6 disambiguation rules identified

### Open Questions
- Exact stratified sampling algorithm for Tier 2 benchmark
- How many columns per type in Tier 2 (originally discussed 500-1000 total)
- Whether Tier 2 needs human review or can be fully algorithmic
- Which rules are most likely to be absorbed by a retrained model
