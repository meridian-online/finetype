---
id: NNFT-130
title: Re-baseline GitTables 1M and SOTAB CTA with CLI pipeline
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-25 08:01'
updated_date: '2026-02-25 08:51'
labels:
  - eval
  - accuracy
  - cli
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GitTables (57.8% domain) and SOTAB (53.7% domain) were last evaluated at v0.1.8 using the DuckDB extension — flat CharCNN, no tiered model, no Model2Vec, no header hints, no attractor demotion, no column disambiguation. Five versions of improvements (v0.1.9–v0.3.0) are untested on these large benchmarks.

Re-run both benchmarks using the v0.3.0 CLI pipeline (tiered model + Model2Vec + all 14+ disambiguation rules + header hints) to establish a true baseline and identify remaining accuracy gaps.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 finetype infer --mode column supports --header flag for column name hint
- [x] #2 finetype infer --mode column supports --batch flag for JSONL batch processing
- [x] #3 Batch mode reads JSONL from stdin with header/values fields and outputs JSONL results
- [x] #4 eval/gittables/eval_cli.py reads column_values.parquet and pipes columns to CLI batch mode
- [x] #5 eval/gittables/eval_cli.sql scores CLI predictions against ground truth (adapted from eval_1m.sql)
- [x] #6 eval/sotab/eval_cli.py reads column_values.parquet and pipes columns to CLI batch mode
- [x] #7 eval/sotab/eval_cli.sql scores CLI predictions against ground truth (adapted from eval_sotab.sql)
- [x] #8 Makefile has eval-1m-cli and eval-sotab-cli targets
- [x] #9 cargo test passes with no regressions
- [x] #10 Existing infer behaviour unchanged (no --header or --batch = same as before)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add --header and --batch flags to the Infer subcommand in main.rs
2. Implement batch JSONL processing in the column mode branch of cmd_infer
3. Wire up semantic hint + taxonomy for batch/header column mode (matching profile cmd)
4. Add unit test for batch mode JSONL round-trip
5. Create eval/gittables/eval_cli.py — reads column_values.parquet, generates JSONL, pipes to CLI
6. Create eval/gittables/eval_cli.sql — scoring SQL adapted from eval_1m.sql (no extension, loads cli_predictions.csv)
7. Create eval/sotab/eval_cli.py — same pattern for SOTAB
8. Create eval/sotab/eval_cli.sql — scoring SQL adapted from eval_sotab.sql
9. Add eval-1m-cli and eval-sotab-cli targets to Makefile
10. Run cargo test, cargo run -- check, verify no regressions
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Step 1 complete: --header and --batch flags added to infer subcommand. Batch mode reads JSONL, classifies with full pipeline (tiered + Model2Vec + taxonomy + attractor demotion), outputs JSONL. Manual testing confirms all three modes work: existing (no flags), --header, --batch.

Steps 2-4 complete: Created eval_cli.py + eval_cli.sql for both GitTables and SOTAB. Makefile has eval-1m-cli and eval-sotab-cli targets. All 263 tests pass, taxonomy check passes, clippy clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added CLI batch mode for column classification and evaluation pipelines to re-baseline GitTables 1M and SOTAB CTA benchmarks with the full v0.3.0 inference pipeline.

Changes:
- Added `--header` and `--batch` flags to `finetype infer --mode column` (main.rs)
  - `--header <NAME>`: provides column name for Model2Vec/hardcoded header hint
  - `--batch`: reads JSONL from stdin (`{\"header\":\"...\",\"values\":[...]}`), outputs JSONL results
  - Batch mode loads full pipeline once (tiered + Model2Vec + taxonomy + attractor demotion) for all columns
  - Also wired semantic hints into non-batch column mode (was previously missing vs profile command)
- Created `eval/gittables/eval_cli.py` — reads column_values.parquet, generates JSONL with header hints, pipes to CLI batch mode, writes cli_predictions.csv
- Created `eval/gittables/eval_cli.sql` — scoring SQL adapted from eval_1m.sql: loads cli_predictions.csv, keeps all scoring logic (sections 4-9), adds header hint impact analysis (section 9)
- Created `eval/sotab/eval_cli.py` — same pattern for SOTAB (no header hints — integer column indices)
- Created `eval/sotab/eval_cli.sql` — scoring SQL adapted from eval_sotab.sql with disambiguation rule analysis
- Added `eval-1m-cli` and `eval-sotab-cli` Makefile targets
- Updated CLAUDE.md CLI commands table and eval infrastructure description

Tests:
- cargo test: 263 tests pass (0 regressions)
- cargo run -- check: all 169 types pass
- cargo clippy: clean
- Manual verification: batch mode, --header flag, and existing behaviour all work correctly"

## Evaluation Results (v0.3.0 CLI pipeline)

### GitTables 1M
- **Format-detectable domain: 56.5%** (v0.1.8 DuckDB: 57.8%, -1.3pp)
- Format-detectable label: 47.1%
- All mapped domain: 49.4% (23,465 columns)
- 45,428 columns, 774,350 values, 32 cols/sec (1,418s)
- 153 unique predicted labels, 46.6% disambiguated
- Top domain: technology 93.5%, datetime 83.5%, identity 60.5%

### SOTAB CTA
- **Format-detectable domain: 54.8%** (v0.1.8 DuckDB: 53.7%, +1.1pp)
- Format-detectable label: 30.5%
- Direct matches domain: 58.2% (5,075 columns)
- All mapped domain: 44.8% (16,765 columns)
- 16,765 columns, 282,278 values, 37 cols/sec (451s)
- 109 unique predicted labels, 22.4% disambiguated
- Top domain: technology 97.3%, datetime 70.0%, geography 62.2%

### Key Findings
1. Domain accuracy roughly comparable — tiered model + disambiguation ≈ flat model at macro level
2. GitTables slight regression (-1.3pp) may be from disambiguation rules overcorrecting some flat-model-correct predictions
3. SOTAB gained +1.1pp purely from disambiguation rules (no header hints available)
4. Biggest accuracy gaps: telephone→categorical (phone validation failures), Duration→sedol, DateTime variant confusion
5. Header hints on GitTables semantic_only tier achieve 51.1% domain — significant value
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [x] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
