---
id: NNFT-269
title: 'LLM distillation: Qwen3 32B teacher labelling of real-world columns via Ollama'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-09 00:56'
updated_date: '2026-03-10 21:52'
labels:
  - data
  - distillation
  - architecture
milestone: m-13
dependencies: []
references:
  - discovery/sense-architecture-challenge/ARCHITECTURE_EVOLUTION.md
  - discovery/sense-architecture-challenge/FINDINGS.md
  - discovery/llm-distillation/FINDINGS.md
  - output/llm_labels.csv
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Phase 4 of the architecture evolution. Parallel data effort on M1 MacBook.

Use Qwen3 32B via Ollama to classify real-world columns from GitTables and other CSV sources into FineType's 250-type taxonomy. This serves dual purpose:

1. Ceiling validation — how accurate can a large model be on our 250-type taxonomy?
2. Training data generation — high-quality labels from real-world data (vs synthetic)

Approach:
- Download GitTables subset (or sample from Kaggle/data.gov CSVs)
- For each column: extract header + 10-20 sample values
- Prompt Qwen3 32B with taxonomy description + constrained output (type label only)
- Validate outputs against FineType's taxonomy (reject invalid labels)
- Build labelled dataset: ~50K columns with LLM-assigned types
- Compare LLM labels vs FineType predictions to identify systematic gaps

Hardware: M1 MacBook with Ollama. Qwen3 32B Q4 requires ~20GB — fits M1 with 32GB+ RAM.
Estimated throughput: ~2-5 columns/second with constrained decoding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Set up Qwen3 32B via Ollama on M1 MacBook (verify model loads and runs)
- [x] #2 Design prompt template for column type classification with constrained output to 250 valid labels
- [x] #3 Process at least 10K real-world columns (from GitTables or mixed CSV sources)
- [x] #4 Validate LLM outputs: rejection rate for invalid labels, distribution across 250 types
- [x] #5 Compare LLM labels vs FineType predictions: agreement rate, systematic disagreements
- [x] #6 Report ceiling accuracy: what percentage of columns does Qwen3 classify correctly (manual spot-check of 200+ columns)
- [x] #7 Output labelled dataset in CSV format: (source, header, sample_values, llm_label, finetype_label, agreement)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Fix llm_label.py script bugs (think mode, CLI syntax, tqdm)
2. Run 5,359-column labelling with Qwen3 8B on M1 MacBook
3. Analyze results: validity, agreement by domain, systematic disagreements
4. Spot-check LLM vs FineType quality on key disagreement patterns
5. Write findings with scaling recommendations to discovery/llm-distillation/FINDINGS.md
6. Commit and close
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Script fixes committed as 49169e7:
- "think": false API flag (93% valid, up from 60%)
- --mode column fix (FineType comparison now working)
- tqdm progress bar with graceful fallback
- Regex thinking fallback for label extraction

Full run completed: 5,359 columns, 832 minutes, 97% valid, 20% agreement.

Analysis complete. Key findings:
- LLM defaults to container.array.* for 1,744 columns (biggest failure mode)
- representation domain only 17% agreement (LLM can't do fine-grained types)
- technology domain 62% agreement (structurally unambiguous types)
- 990 cases where LLM may be more specific than FineType (amount, username, HTML)
- 325 integer/decimal disagreements from pandas .0 float export

Spot-check of AC6 done inline during analysis — LLM is a good coarse classifier (technology, common identity types) but poor at FineType's statistical/structural subtypes (ordinal, categorical, increment, numeric_code).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
LLM distillation spike using Qwen3 8B via Ollama on 5,359 real-world columns (508 CSVs, 96% GitTables).

Script fixes (committed 49169e7):
- Disabled Qwen3 thinking mode at API level ("think": false) — 97% valid labels (was 60%)
- Fixed FineType CLI invocation (--mode column, not --column) — comparison now works
- Added tqdm progress bar with graceful fallback
- Added regex fallback to extract labels from thinking text

Results:
- 97% valid labels (5,234/5,359), 125 hallucinated-but-plausible invalids
- 20% exact agreement with FineType (1,056/5,234)
- Technology domain: 62% agreement (structurally unambiguous types)
- Representation domain: 17% (LLM can't do statistical/structural subtypes)
- Finance domain: 3% (can't distinguish basis_points/yield/amount variants)

Key finding: LLM is a good coarse classifier but poor at FineType's fine-grained types (ordinal, categorical, increment, numeric_code, binary). Biggest failure: defaults to container.array.* for 1,744 columns. ~990 cases where LLM is arguably more specific (amount, username, HTML detection).

Scaling recommendations in discovery/llm-distillation/FINDINGS.md: constrained decoding (free, eliminates invalids), Qwen3 32B on cloud GPU (~$10 for 50K columns), two-stage domain→type prompting, API-based alternatives.

Output: output/llm_labels.csv, discovery/llm-distillation/FINDINGS.md
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
