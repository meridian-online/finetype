---
id: NNFT-216
title: Wire JSON profiling into CLI with structured output
status: To Do
assignee: []
created_date: '2026-03-04 20:15'
updated_date: '2026-03-06 00:00'
labels:
  - cli
  - json
milestone: m-9
dependencies:
  - NNFT-209
references:
  - crates/finetype-cli/src/main.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extend `finetype profile` command to handle JSON/NDJSON input with structured output. Auto-detects file format by extension, reads via json_reader module, classifies each path using ColumnClassifier (with path leaf as header hint), and produces output that preserves JSON structure for JSON format while flattening for plain/CSV.

Key output distinction:
- JSON output: reconstructs nested structure showing full path hierarchy
- Plain/CSV output: flat list of paths with classification results (natural for terminals/pipelines)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 JSON/NDJSON auto-detection by .json/.ndjson/.jsonl extension
- [ ] #2 json_reader module used for both single documents and NDJSON line-by-line reading
- [ ] #3 Path leaf used as header hint for ColumnClassifier (e.g., 'email' for 'users[].email')
- [ ] #4 JSON output reconstructs nested structure showing full path hierarchy with type/confidence at each level
- [ ] #5 Plain output shows flat list of paths (natural for terminal/pipeline consumption)
- [ ] #6 CSV output includes path, type, broad_type, confidence columns
- [ ] #7 All output formats work correctly with JSON input files
- [ ] #8 Clear error message on malformed JSON input
- [ ] #9 Depends on NNFT-209 (json_reader module)
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
