# Interview: Distillation v3 — Wild Expansion

**Date:** 2026-03-18
**Interviewer:** Nightingale

---

## Context

Phase 2 distillation completed: 5,364 columns from 507 CSV files (GitTables + curated eval datasets) classified via blind-first adjudication using Claude Sonnet agents. 36% agreement with FineType, with 10 systematic gap categories identified.

The GitTables corpus is dominated by HN discussion dumps and software metrics datasets, which trigger the same FineType weaknesses repeatedly. Hugh wants to validate FineType against diverse real-world data "from the wild" before implementing fixes — to ensure the fix priority list is correct.

## Interview Q&A

### Q1: Sequencing
**Q:** Should we implement top fixes first, or run Phase 3 on wild data to validate the priority list?
**A:** Phase 3 first (Option A). Wild datasets will tell us whether the fix priority list from Phase 2 is right before we invest in implementing fixes.

### Q2: Motivation
**Q:** What convinced you that wild data is the priority?
**A:** The earthquakes dataset from the disambiguator spike. Real-world data surfaces different patterns than curated benchmarks. The more we include data "from the wild" into our eval, the more robust inference will be.

### Q3: Scope
**Q:** How many datasets, from where?
**A:** 20-30 diverse datasets from multiple sources. Not discussed in detail yet — to be refined at session start.

---

## Summary

### Goal
Run the distillation pipeline on 20-30 diverse real-world datasets from outside GitTables to validate and potentially reorder the fix priority list from Phase 2.

### Constraints
- Same blind-first adjudication pipeline as Phase 2
- Same batch agent infrastructure (scripts already committed)
- Claude Max 20x subscription for agent compute
- Beelink mini PC (16GB RAM, 5 agents max)

### Success Criteria
- 20+ datasets from 3+ distinct sources (Kaggle, gov open data, ML repos)
- Domain diversity: at least 5 distinct domains (healthcare, geoscience, transport, economics, etc.)
- Comparison of gap patterns against Phase 2 findings
- Updated fix priority list if wild data changes the ranking

### Data Sources

All datasets live under `/home/hugh/datasets/`.

**Already local:**

| Source | Location | Format | Status |
|--------|----------|--------|--------|
| GitTables | `/home/hugh/datasets/gittables/` | CSV (extracted from parquet) | ✅ Phase 2 done (507/509 files) |
| SOTAB CTA | `/home/hugh/datasets/sotab/cta/` | JSON tables (gzipped) | ❌ Not yet in distillation |
| Earthquakes | `/home/hugh/datasets/earthquakes_2024.csv` | CSV | ❌ Not yet in distillation |
| Curated eval | `/home/hugh/datasets/*.csv` (21 files) | CSV | ✅ Phase 2 done |

**SOTAB details:**
- Validation split: 5,732 JSON table files in `/home/hugh/datasets/sotab/cta/validation/Validation/`
- Test split: available as `CTA_Test.zip`
- Ground truth: `CTA_validation_gt.csv` (table_name, column_index, label — Schema.org types)
- Pre-extracted: `column_values.parquet` may already have column data ready
- Needs JSON→JSONL extraction step (different from CSV pipeline)

**To source for Phase 3:**

| Source | Domain coverage | Access |
|--------|----------------|--------|
| Kaggle | Healthcare, sports, e-commerce, social | API or manual download |
| data.gov.au / ABS | Census, economic indicators, public services | Direct CSV download |
| data.gov (US) | Transport, environment, education, federal | Direct CSV download |
| UCI ML Repository | Classic ML datasets with known schemas | Direct download |
| World Bank Open Data | Economic development, demographics | CSV export |

### Phase 3 Tracks

**Track A — SOTAB distillation:** Extract columns from SOTAB JSON tables, run through blind-first adjudication. Compare Claude labels against both FineType and SOTAB ground truth (Schema.org labels). This gives us a three-way comparison.

**Track B — Wild CSV sourcing:** Download 20-30 diverse CSVs from Kaggle/gov/UCI, add to `data/csvs/`, run through existing pipeline. Include earthquakes_2024.csv.

### Open Questions
- Exact dataset selection criteria (file size limits? column count?)
- Whether to include non-English datasets (multilingual.csv already in Phase 2)
- SOTAB extraction: use existing `column_values.parquet` or re-extract from JSON?
- Whether to run FineType fixes between Phase 2 and Phase 3 or keep the same baseline
- How to map Schema.org labels to FineType taxonomy for three-way comparison
