---
id: NNFT-184
title: 'Phase B: Evaluation Rust - Replace 7 Python eval scripts with Rust binaries'
status: In Progress
assignee:
  - '@eval-engineer'
created_date: '2026-03-02 07:23'
updated_date: '2026-03-02 07:25'
labels:
  - phase-b
  - evaluation
  - large
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Port evaluation infrastructure (eval_report.py, eval_actionability.py, GitTables/SOTAB eval scripts) to pure Rust.

**Objective**: Eliminate venv requirement for evaluation targets. Pure Rust evaluation suite with identical output.

**Work**:
1. Create `crates/finetype-eval/` crate with binaries:
   - `eval-report` — Markdown dashboard aggregation (CSVs + YAML config → report.md)
   - `eval-extract` — GitTables 1M metadata extraction
   - `eval-prepare-values` — Value sampling from Parquet
   - `eval-cli` variants for GitTables and SOTAB

2. Dependencies:
   - `csv`, `arrow2` or `parquet` for I/O
   - `serde_json` for aggregation
   - `duckdb` Rust binding for optional SQL evaluation

3. Migrate Makefile eval targets from Python to Rust binaries
4. Validate output matches current Python implementation (CSVs, JSON, markdown)

**Acceptance criteria**:
- `make eval-report` produces identical markdown to current version
- `make eval-extract`, `make eval-prepare-values` produce identical CSVs/JSONs
- `make eval-1m-cli`, `make eval-sotab-cli` work without venv
- No Python venv setup required for any evaluation target
- All eval make targets pass

**Note**: Can run in parallel with Phase 0 spike. Largest independent workstream (~20-30 hours).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Create finetype-eval crate with csv + arrow2 dependencies
- [ ] #2 Implement eval-report binary with markdown aggregation logic
- [ ] #3 Implement eval-extract and eval-prepare-values binaries
- [ ] #4 Update Makefile eval targets to call Rust binaries instead of Python
- [ ] #5 Validate eval-report output matches current Python version exactly
- [ ] #6 Validate eval-extract, eval-prepare-values CSV output matches
- [ ] #7 Test eval-1m-cli and eval-sotab-cli without venv
- [ ] #8 Update DEVELOPMENT.md with pure Rust evaluation workflow
- [ ] #9 Verify make eval passes without Python
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Implementation Plan

### Phase 1: Crate setup + eval-report binary (AC #1, #2)
1. Create `crates/finetype-eval/` with Cargo.toml (multi-binary crate)
2. Add workspace dependencies: csv, serde_yaml, serde_json, clap, chrono, anyhow
3. Add to workspace Cargo.toml members list (but NOT default-members, to keep main build fast)
4. Implement `eval-report` binary — port eval_report.py:
   - Load CSVs (profile_results, ground_truth, schema_mapping, actionability_results)
   - Load taxonomy YAML stats (types, format_string, validation counts per domain)
   - Implement is_label_match / is_domain_match interchangeability rules
   - Compute profile accuracy metrics + precision per type
   - Generate markdown report matching current format exactly
5. Also implement `eval-mapping` binary — port the Python one-liner in Makefile (YAML → CSV converter)
6. Validate output matches current report.md

### Phase 2: eval-actionability binary (AC #2 continued)
1. Implement `eval-actionability` binary — port eval_actionability.py
2. Use duckdb Rust crate for TRY_STRPTIME testing (same approach as Python)
3. Load manifest, predictions, format_strings from taxonomy YAML
4. Run DuckDB queries on CSV files, write results CSV + console report

### Phase 3: GitTables binaries (AC #3)
1. `eval-extract` — port extract_metadata_1m.py (uses parquet crate to read GitTables metadata)
2. `eval-prepare-values` — port prepare_1m_values.py (read parquet, sample column values, write parquet)
3. `eval-gittables-cli` — port gittables/eval_cli.py (read parquet via duckdb, subprocess finetype CLI, write CSV)

### Phase 4: SOTAB binaries (AC #3 continued)
1. `eval-sotab-prepare` — port sotab/prepare_values.py (read gzipped JSON, write parquet)
2. `eval-sotab-cli` — port sotab/eval_cli.py (read parquet via duckdb, subprocess finetype CLI, write CSV)

### Phase 5: profile_eval.sh Python removal
1. Replace inline Python JSON parsing in profile_eval.sh with jq or a small Rust binary (`eval-profile-parse`)
2. Alternative: port the entire profile_eval.sh to a Rust binary that calls finetype profile

### Phase 6: Makefile + validation (AC #4-#9)
1. Update Makefile eval targets: replace $(VENV_PYTHON) calls with cargo run -p finetype-eval
2. Remove VENV_PYTHON variable from Makefile
3. Run all eval targets end-to-end, diff outputs vs Python versions
4. Update DEVELOPMENT.md (or create if needed)
5. Update CLAUDE.md Current State if appropriate

### Dependencies
- `csv` (already in workspace)
- `serde_yaml` (already in workspace)
- `serde_json` (already in workspace)
- `clap` (already in workspace)
- `chrono` (already in workspace)
- `anyhow` (already in workspace)
- `parquet` + `arrow` crates (NEW — for reading/writing parquet files)
- `duckdb` (already in workspace — for actionability TRY_STRPTIME, reading parquet in CLI evals)
- `flate2` (NEW — for reading gzipped SOTAB JSON files)
- `rand` (already in workspace — for sampling)

### Order of implementation
Start with eval-report (most impactful, most testable), then work outward.
eval-actionability and profile scripts can be validated independently.
GitTables/SOTAB scripts require large external datasets — validate format only.
<!-- SECTION:PLAN:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
