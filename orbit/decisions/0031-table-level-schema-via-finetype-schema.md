---
status: accepted
date-created: 2026-03-12
date-modified: 2026-03-12
---
# 0031. Table-level schema via finetype schema <file>

## Context and Problem Statement

`finetype schema` only handled type-level keys (e.g., `identity.person.email`). Users needed a way to generate table-level JSON Schema from CSV data — column names mapped to types with validation constraints — to enable repeatable validation workflows.

## Considered Options

- Add a `--schema` flag to `profile` to emit table-level schema alongside profiling
- Extend `schema` to detect file paths and run profile internally
- Create a separate `schema table` subcommand

## Decision Outcome

Chosen option: "Extend schema to detect file paths", because it keeps the CLI surface minimal (one `schema` command for all schema operations) and follows the principle of context-dependent behaviour. The command detects whether the argument is a file path (by extension + existence) or a type key, routing accordingly.

### Consequences

- Good, because one command handles both type-level and table-level schemas
- Good, because `--stats` flag adds observed data constraints (min/max/cardinality) as opt-in
- Bad, because the detection heuristic (file extension check) could theoretically misroute if a type key collides with a filename
