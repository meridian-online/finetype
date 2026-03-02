---
id: NNFT-186
title: 'Phase D (Path A): Cleanup - Remove Python infrastructure'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-02 07:23'
updated_date: '2026-03-02 23:22'
labels:
  - phase-d
  - cleanup
  - path-a-only
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**BLOCKED until Phase C (Path A) completes.**

Final cleanup after Candle training port. Remove all Python infrastructure and polish documentation.

**Objective**: Achieve pure Rust codebase with zero Python files (except test artifacts).

**Work**:
1. Delete Python infrastructure:
   - Remove `.venvs/` directories
   - Delete `scripts/` Python utilities (merge, fix taxonomy, etc.)
   - Remove Python eval/training files (already handled in Phases B/C)

2. Update documentation:
   - `DEVELOPMENT.md` — Remove Python venv setup; clarify pure Rust workflow
   - `README.md` — Update to reflect "Pure Rust" instead of "Rust + Python"
   - `Makefile` — Remove all Python venv references

3. CI/CD verification:
   - Confirm GitHub Actions requires no Python
   - All test/build targets use Rust only

**Acceptance criteria**:
- Repository contains zero Python files (except test/discovery artifacts)
- CI/CD requires no Python installation
- DEVELOPMENT.md accurately documents pure Rust workflow
- README.md clarifies "pure Rust" implementation
- No `.venvs/` or Python setup scripts remain in repo

**Note**: Do NOT start until Phase C (Path A) completes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All Python training scripts deleted (train_sense, train_entity, prepare_sense, prepare_model2vec)
- [x] #2 Remove .venvs/ and venv setup scripts
- [x] #3 Delete other Python utilities (scripts/*.py except discovery artifacts)
- [x] #4 Update DEVELOPMENT.md with pure Rust workflow (no venv)
- [x] #5 Update README.md to state 'Pure Rust' architecture
- [x] #6 Remove Python venv references from Makefile
- [x] #7 Verify GitHub Actions CI requires no Python
- [x] #8 Confirm all tests pass without Python
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Delete eval Python files replaced by Rust (7 files)
2. Delete scripts/*.py one-off utilities (4 files)
3. Delete __pycache__ directories (3 dirs)
4. Check Makefile for Python venv references
5. Verify CI requires no Python
6. Update README.md
7. Run tests
8. Commit
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Removed all Python infrastructure from the finetype codebase, completing Phase D of the Pure Rust Return.

Deleted:
- 7 eval Python files (all replaced by Rust binaries in NNFT-184): eval_report.py, eval_actionability.py, extract_metadata_1m.py, prepare_1m_values.py, eval_cli.py (×2), prepare_values.py
- 4 scripts/*.py one-off utilities: merge_taxonomy.py, fix_taxonomy.py, extract_cldr_patterns.py, compare_sense_vs_finetype.py
- 3 __pycache__ directories with compiled bytecode

Kept (discovery artifacts):
- 5 .py files in discovery/ (research spikes for entity disambiguation and Model2Vec specialisation)

Updated:
- README.md: crate list 4→7, repo structure reflects eval/train crates, type count 166

Verified:
- No .venvs/, requirements.txt, or Python venv references exist
- CI workflows require no Python
- Makefile python3 calls limited to DuckDB extension build (NNFT-183 scope)
- 253 tests pass, taxonomy check passes
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
