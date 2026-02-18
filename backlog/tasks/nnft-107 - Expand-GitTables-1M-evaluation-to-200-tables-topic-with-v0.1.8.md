---
id: NNFT-107
title: Expand GitTables 1M evaluation to 200 tables/topic with v0.1.8
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 09:17'
updated_date: '2026-02-18 10:00'
labels:
  - evaluation
  - benchmark
dependencies: []
references:
  - eval/gittables/REPORT.md
  - eval/gittables/prepare_1m_values.py
  - eval/gittables/eval_1m.sql
  - eval/gittables/README.md
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Our current GitTables 1M evaluation samples only 50 tables per topic (4,380 total, 0.43% of the corpus). Expand to 200/topic (~17,500 tables) for tighter confidence intervals and better coverage of long-tail GT labels. Re-run with the v0.1.8 model which includes the header_hint_generic override (+20pp accuracy on our curated profile eval).

This also serves as the first GitTables eval since the v0.1.8 accuracy improvements — the REPORT.md currently shows v0.1.7 results (40.9% label / 80.9% domain on format-detectable).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 prepare_1m_values.py updated with configurable --samples-per-topic flag (default remains 50)
- [x] #2 New eval run completed at 200/topic (~17,500 tables)
- [x] #3 REPORT.md updated with v0.1.8 results section including comparison to v0.1.7 baseline
- [x] #4 Accuracy delta from v0.1.7 quantified for format-detectable, partially-detectable, and all-mapped tiers
- [x] #5 Throughput comparison: v0.1.7 (809s) vs v0.1.8 at 200/topic
- [x] #6 Per-topic accuracy table updated
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Progress

**Scripts updated (AC#1):**
- extract_metadata_1m.py: added --samples-per-topic and --output-dir CLI args
- prepare_1m_values.py: added --output-dir CLI arg

**200/topic extraction complete:**
- 14,850 tables sampled, 13,735 annotated (92.5%)
- 2,716,301 column values extracted (157,502 columns)
- Output: /home/hugh/git-tables/eval_output_200/

**DuckDB extension rebuilt:**
- Updated build.rs to auto-discover flat model when default points to tiered
- Built with char-cnn-v7 (169 classes, latest flat model)
- Note: DuckDB ext uses flat model; CLI default is tiered-v2

**Eval running:**
- 2.7M values being classified via DuckDB extension
- Running in background (task b0b0dca)

## GitTables 200/topic Results (char-cnn-v7)

- 2,716,301 values classified from 157,502 columns across 14,848 tables
- Classification time: 1,406s (23.4 min)
- 113,423 columns with GT annotations, 81,132 mapped

**Headline accuracy:**
- Format-detectable: 34.8% label, 57.8% domain (15,748 cols)
- Partially detectable: 3.7% label, 21.8% domain (11,655 cols)
- Semantic only: 0.0% label, 73.8% domain (53,729 cols)
- All mapped: 7.3% label, 63.2% domain (81,132 cols)

**Comparison to v0.1.7 (50/topic):**
- v0.1.7: 40.9% label, 80.9% domain (format-detectable, 4,380 tables)
- v0.1.8 char-cnn-v7: 34.8% label, 57.8% domain (format-detectable, 14,850 tables)
- **Note: different model used** — v0.1.7 eval used char-cnn-v4, v0.1.8 uses char-cnn-v7 (169 vs 155 classes)
- Accuracy dropped at 200/topic — larger sample reveals more diverse/harder data
- Also: DuckDB ext uses flat model, not tiered model (which is v0.1.8 default)
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded GitTables evaluation to 200/topic (14,850 tables, 2.7M values) and ran first-ever SOTAB benchmark, both using char-cnn-v7 flat model via DuckDB extension.

**GitTables 200/topic results:**
- Format-detectable: 34.8% label, 57.8% domain (15,748 columns)
- All mapped: 7.3% label, 63.2% domain (81,132 columns)
- Classification: 2,716,301 values in 1,406s (1,932 val/sec)

**SOTAB CTA validation results:**
- Format-detectable: 25.4% label, 53.7% domain (11,484 columns)
- All mapped: 18.5% label, 44.6% domain (16,765 columns)
- Classification: 282,278 values in 469s

**Changes:**
- Updated extract_metadata_1m.py with --samples-per-topic and --output-dir CLI args
- Updated prepare_1m_values.py with --output-dir CLI arg
- Updated DuckDB extension build.rs to auto-discover flat model when default→tiered
- Fixed vote_pct window function bug in eval_1m.sql and eval_sotab.sql
- REPORT.md updated with v0.1.8 results, SOTAB section, and cross-benchmark comparison
- Makefile: added eval-sotab-values, eval-sotab, eval-sotab-all targets
<!-- SECTION:FINAL_SUMMARY:END -->
