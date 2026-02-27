---
id: NNFT-149
title: CLI full taxonomy export and schema command
status: In Progress
assignee:
  - '@nightingale'
created_date: '2026-02-27 02:55'
updated_date: '2026-02-27 03:19'
labels:
  - cli
  - taxonomy
  - schema
dependencies: []
references:
  - discovery/cli-types/cli-full-export-and-schema-command.md
  - crates/finetype-cli/src/main.rs
  - crates/finetype-core/src/taxonomy.rs
  - crates/finetype-core/src/validator.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add --full flag to finetype taxonomy for complete type export, and a new finetype schema subcommand for per-type JSON Schema extraction.

Context: The website type registry (NNWB-015) needs complete taxonomy data. Currently finetype taxonomy exports only 7 of 16+ fields. Analysts also want per-type JSON Schema for validation pipelines. Both gaps are serialisation work — the data is already loaded at runtime.

Two deliverables:
1. finetype taxonomy --full --output json — exports all fields (description, validation, samples, decompose, aliases, references, format_string, transform_ext, tier, etc.)
2. finetype schema <type_key> — outputs enriched JSON Schema with $id, title, description, examples. Supports --pretty and glob patterns.

Discovery brief: discovery/cli-types/cli-full-export-and-schema-command.md
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 finetype taxonomy --full --output json exports all 16+ fields per type
- [x] #2 finetype schema <type_key> outputs a valid JSON Schema document
- [x] #3 finetype schema <type_key> --pretty outputs formatted JSON
- [x] #4 finetype schema "domain.category.*" supports glob patterns
- [x] #5 Unknown type key returns exit code 1 with helpful error
- [x] #6 Existing finetype taxonomy output (without --full) is unchanged
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add --full flag to Taxonomy command in CLI args struct
2. Extend cmd_taxonomy() JSON path: when --full, serialize all Definition fields
   - Convert serde_yaml::Value fields (samples, references, decompose) to serde_json::Value
   - Serialize validation as JSON Schema via to_json_schema()
   - Parse key into domain/category/type components
   - Format designation as lowercase string (not Debug)
3. Add Schema subcommand to Commands enum with type_key positional arg + --pretty flag
4. Implement cmd_schema():
   - Parse type_key, support glob with * (e.g. "identity.person.*")
   - Look up via Taxonomy::get() for exact match, or Taxonomy::by_category()/by_domain() for globs
   - Build enriched JSON Schema: merge validation.to_json_schema() with $schema, $id, title, description, examples
   - Single type → object output, multiple types → array output
   - --pretty for formatted, compact by default
   - Exit code 1 + helpful error for unknown type
5. Add tests: verify --full has all fields, verify schema output is valid JSON Schema
6. Verify AC#6: existing finetype taxonomy output unchanged
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
All 6 AC verified:
- AC#1: 19 fields in --full export (inc. validation_by_locale on 5 types)
- AC#2: JSON Schema with $schema, $id, title, description, pattern, examples
- AC#3: --pretty produces formatted output
- AC#4: Glob patterns work (identity.person.* → 16 types, datetime.* → 46 types)
- AC#5: Exit code 1 with Levenshtein-based suggestions (emal → email)
- AC#6: Existing 7-field output unchanged
- Clippy clean, 305 tests pass
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added --full flag to finetype taxonomy and new finetype schema subcommand for complete type export and JSON Schema extraction.

Changes:
- finetype taxonomy --full --output json exports 19 fields per type (was 7): domain, category, type, description, validation (as JSON Schema), validation_by_locale, samples, format_string, decompose, tier, aliases, references, notes, etc.
- finetype schema <type_key> outputs enriched JSON Schema with $schema, $id, title, description, validation keywords, and examples from samples
- finetype schema <type_key> --pretty for formatted output (compact by default)
- finetype schema "domain.category.*" supports glob patterns for multi-type export
- Unknown type key exits with code 1 and suggests similar types via Levenshtein distance
- Existing finetype taxonomy output (without --full) is unchanged (backward compatible)

Implementation:
- yaml_to_json conversion via serde_json::to_value() (no new dependencies)
- Designation serialized as snake_case string (not Debug format) in --full mode
- Glob matching uses prefix-based string matching (no regex dependency)
- build_json_schema() enriches validation.to_json_schema() with JSON Schema metadata

Tests: cargo test 305 pass, cargo clippy clean, all 6 AC verified
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [x] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [x] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [x] #4 Decision record created if plan involved choosing between approaches
- [x] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
