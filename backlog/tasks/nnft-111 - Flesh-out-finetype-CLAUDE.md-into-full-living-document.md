---
id: NNFT-111
title: Flesh out finetype CLAUDE.md into full living document
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-24 01:50'
updated_date: '2026-02-24 01:50'
labels:
  - documentation
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The finetype CLAUDE.md was a skeleton with just Noon Pillars and Backlog Discipline. It needs to be a comprehensive living document like other project CLAUDE.md files, covering: Current State, Architecture, Priority Order, Decided Items, Build & Test, and Key File Reference.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Current State section with version, taxonomy count, model info, recent milestones, in-progress work
- [x] #2 Architecture section with workspace layout, crate dependency graph, inference pipeline, tiered model, taxonomy structure, DuckDB functions, CLI commands, eval infrastructure
- [x] #3 Priority Order section reflecting current backlog priorities
- [x] #4 Decided Items section capturing key architectural decisions with task references
- [x] #5 Build & Test section with common commands
- [x] #6 Key File Reference table for quick navigation
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Expanded finetype/CLAUDE.md from a 54-line skeleton (Noon Pillars + Backlog Discipline) into a comprehensive 220-line living document.

New sections added:
- **Current State** — v0.1.8, 169 types, tiered-v2 default model, distribution channels, recent milestones, in-progress work
- **Architecture** — Workspace layout, crate dependency graph (core -> model -> cli/duckdb), inference pipeline (flat vs tiered, column-mode disambiguation, header hints), tiered model T0->T1->T2 architecture, taxonomy structure (6 domains, transformation contracts), DuckDB extension functions (7 scalar functions), CLI commands, evaluation infrastructure (profile, GitTables, SOTAB)
- **Priority Order** — Ranked from DuckDB extension through documentation, distribution, data quality, to new domains
- **Decided Items** — 10 key architectural decisions with task references (tiered model, taxonomy format, YAML contracts, CharCNN, column disambiguation, DuckDB flat model, HuggingFace hosting, boolean restructure, pre-commit hooks, eval config)
- **Build & Test** — Common development commands
- **Key File Reference** — Quick lookup table for 12 key files/directories

All content grounded in actual codebase exploration: taxonomy check output (169 defs, 6 domains, 100% pass), Cargo.toml (workspace version, crate members), model directory structure (34 tier subdirs), CHANGELOG entries, eval reports, and source file analysis.
<!-- SECTION:FINAL_SUMMARY:END -->
