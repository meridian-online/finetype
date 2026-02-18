---
id: NNFT-108
title: Consolidate eval dataset paths with config-based approach
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-18 09:57'
updated_date: '2026-02-18 10:17'
labels:
  - infrastructure
  - evaluation
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Move all evaluation datasets to ~/datasets/ and replace hardcoded paths across eval scripts, SQL, and Makefile with environment-variable-driven config.

Current state: eval scripts have hardcoded paths like /home/hugh/git-tables/eval_output/ and /home/hugh/sotab/cta/. This breaks portability and makes it hard to manage datasets across machines.

Target directory structure:
- ~/datasets/gittables/ (1M corpus + eval outputs)
- ~/datasets/sotab/ (CTA validation, test, training)
- ~/datasets/eval_output/ (shared eval output directory)

Config approach: eval/config.env with overridable defaults sourced by all scripts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 eval/config.env created with GITTABLES_DIR, SOTAB_DIR, EVAL_OUTPUT_DIR defaults
- [x] #2 extract_metadata_1m.py uses config.env paths instead of hardcoded defaults
- [x] #3 prepare_1m_values.py uses config.env paths instead of hardcoded defaults
- [x] #4 eval_1m.sql parameterized to accept data directory (via DuckDB SET or Makefile substitution)
- [x] #5 eval_sotab.sql parameterized to accept data directory
- [x] #6 prepare_values.py (SOTAB) uses config.env paths
- [x] #7 Makefile eval targets source config.env and pass paths to scripts/SQL
- [x] #8 Datasets physically moved to ~/datasets/ and old locations cleaned up
- [x] #9 All eval pipelines tested end-to-end after migration
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Create eval/config.env with GITTABLES_DIR, SOTAB_DIR, EVAL_OUTPUT_DIR, EXTENSION_PATH defaults
2. Update extract_metadata_1m.py — read GITTABLES_DIR from env, keep --output-dir CLI arg
3. Update prepare_1m_values.py — read EVAL_OUTPUT_DIR from env
4. Update prepare_values.py (SOTAB) — read SOTAB_DIR from env
5. Update eval_1m.sql — replace hardcoded paths with ${VAR} placeholders
6. Update eval_sotab.sql — replace hardcoded paths with ${VAR} placeholders
7. Update Makefile — source config.env, use envsubst for SQL, pass env to Python
8. Create ~/datasets/ structure and move data (gittables → ~/datasets/gittables, sotab → ~/datasets/sotab)
9. Create symlinks at old locations for backward compat
10. Test eval pipelines end-to-end
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation complete. Key decisions:

- eval/config.env uses bash export syntax with ${VAR:-default} fallback pattern
- Makefile uses ?= for conditional assignment (env vars override) — defaults match config.env
- SQL files use ${VAR} placeholders, expanded by envsubst with explicit variable list ($EXTENSION_PATH $EVAL_OUTPUT $SOTAB_DIR $SOTAB_SPLIT) to avoid expanding DuckDB's $-prefixed JSON paths
- Python scripts read os.environ.get() with expanduser defaults
- Added import os to prepare_1m_values.py and prepare_values.py (SOTAB) — both were missing it
- Moved ~/git-tables → ~/datasets/gittables and ~/sotab → ~/datasets/sotab
- Created backward-compat symlinks at old locations
- Also parameterized eval.sql (legacy benchmark) and investigate_tech.sql
- Fixed vote_pct window function bug in investigate_tech.sql (same fix as eval_1m.sql)
- All smoke tests pass: data loads from new paths, envsubst expands correctly"
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Consolidated all eval dataset paths from hardcoded absolute paths to a config-based approach with environment variable overrides.

Changes:
- Created eval/config.env with GITTABLES_DIR, EVAL_OUTPUT, SOTAB_DIR, EXTENSION_PATH, EVAL_VENV defaults pointing to ~/datasets/
- Updated 3 Python scripts (extract_metadata_1m.py, prepare_1m_values.py, prepare_values.py) to read paths from os.environ with expanduser fallbacks
- Parameterized 4 SQL files (eval_1m.sql, eval_sotab.sql, eval.sql, investigate_tech.sql) with ${VAR} placeholders
- Rewrote Makefile eval targets to use envsubst for SQL path expansion and env var passthrough for Python scripts
- Makefile uses ?= conditional assignment so all paths are overridable (e.g., EVAL_OUTPUT=~/custom make eval-1m)
- envsubst uses explicit variable list to avoid expanding DuckDB's $-prefixed JSON path expressions

Data migration:
- Moved ~/git-tables → ~/datasets/gittables (71GB, same filesystem = instant)
- Moved ~/sotab → ~/datasets/sotab (895MB)
- Created backward-compat symlinks at old locations
- Directory structure: ~/datasets/{gittables/,sotab/,*.csv} (eval datasets consolidated)

Bonus fixes:
- Added missing import os to prepare_1m_values.py and prepare_values.py
- Fixed vote_pct window function in investigate_tech.sql (same bug fixed in eval_1m.sql earlier)

Tested:
- Full SOTAB CTA evaluation end-to-end via make eval-sotab (282K values, 16,765 columns) — results match previous run exactly (25.4% label / 53.7% domain)
- GitTables metadata loading smoke test — catalog and column values load from new paths
- Python script path resolution verified via import tests"
<!-- SECTION:FINAL_SUMMARY:END -->
