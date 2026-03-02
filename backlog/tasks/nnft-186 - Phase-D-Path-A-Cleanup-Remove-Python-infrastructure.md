---
id: NNFT-186
title: 'Phase D (Path A): Cleanup - Remove Python infrastructure'
status: To Do
assignee: []
created_date: '2026-03-02 07:23'
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
- [ ] #1 All Python training scripts deleted (train_sense, train_entity, prepare_sense, prepare_model2vec)
- [ ] #2 Remove .venvs/ and venv setup scripts
- [ ] #3 Delete other Python utilities (scripts/*.py except discovery artifacts)
- [ ] #4 Update DEVELOPMENT.md with pure Rust workflow (no venv)
- [ ] #5 Update README.md to state 'Pure Rust' architecture
- [ ] #6 Remove Python venv references from Makefile
- [ ] #7 Verify GitHub Actions CI requires no Python
- [ ] #8 Confirm all tests pass without Python
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
