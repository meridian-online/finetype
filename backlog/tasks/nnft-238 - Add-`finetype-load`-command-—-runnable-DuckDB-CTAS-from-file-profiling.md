---
id: NNFT-238
title: Add `finetype load` command — runnable DuckDB CTAS from file profiling
status: To Do
assignee:
  - '@nightingale'
created_date: '2026-03-07 01:18'
labels:
  - cli
  - feature
dependencies: []
references:
  - crates/finetype-cli/src/main.rs
  - labels/definitions_representation.yaml
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The current `schema-for` command outputs a CREATE TABLE where every column is VARCHAR with FineType types as comments. This doesn't help analysts actually load data — they can't copy-paste-run it.

New `finetype load <file>` command that outputs a runnable DuckDB CREATE TABLE AS SELECT statement using the taxonomy's `broad_type` and `transform` fields to produce properly typed columns.

Example output:
```sql
CREATE TABLE titanic AS
SELECT
    CAST(PassengerId AS BIGINT) AS PassengerId,  -- representation.identifier.increment
    Survived,                                     -- representation.boolean.binary
    Name,                                         -- identity.person.full_name
    CAST(Fare AS DOUBLE) AS Fare,                -- representation.numeric.decimal_number
    Embarked                                      -- representation.discrete.categorical
FROM read_csv('titanic.csv', auto_detect=true);
```

Design decisions from interview:
- CTAS over CREATE+INSERT or bare DDL — single runnable statement
- VARCHAR columns use bare column reference (no redundant CAST)
- File path in read_csv() = exactly what the user provided
- Table name = sanitised filename stem (or --table-name override)
- SQL-only output — no -o json/arrow formats
- Trust model predictions as-is — no confidence guards or VARCHAR fallback
- DuckDB-only target (no --target flag)
- Full dotted labels (domain.category.type) as SQL comments
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 finetype load -f titanic.csv outputs a valid, runnable DuckDB CTAS statement
- [ ] #2 Columns with non-VARCHAR broad_type use the taxonomy transform expression in the SELECT
- [ ] #3 VARCHAR columns appear as bare column references (no CAST(x AS VARCHAR))
- [ ] #4 Each column has a -- domain.category.type comment with the full FineType label
- [ ] #5 Table name defaults to sanitised filename stem; --table-name flag overrides
- [ ] #6 read_csv() uses the exact file path provided by the user
- [ ] #7 Command accepts same model/pipeline flags as profile (--model, --sharp-only, --no-header-hint, --sample-size, --delimiter)
- [ ] #8 Smoke test: load output for a test CSV can be executed in DuckDB without errors
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
