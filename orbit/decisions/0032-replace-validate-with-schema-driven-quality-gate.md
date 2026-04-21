---
status: accepted
date-created: 2026-03-12
date-modified: 2026-03-12
---
# 0032. Replace validate command with schema-driven quality gate

## Context and Problem Statement

The existing `finetype validate` command did per-value repair (quarantine/null/ffill/bfill strategies), conflating quality assessment with data transformation. The `profile --validate` flag also mixed discovery with enforcement. Users needed a clean quality gate that could be used in CI/CD pipelines.

## Considered Options

- Keep old validate and add a new `validate-schema` subcommand
- Rename old validate to `repair`, add new schema-driven `validate`
- Replace old validate entirely with schema-driven quality gate

## Decision Outcome

Chosen option: "Replace entirely", because validation is pass/fail — a quality gate, not a repair tool. The repair strategies (ffill/bfill/null) were an experiment that saw no adoption. Clean separation: `schema` generates, `validate` enforces, `profile` discovers.

### Consequences

- Good, because the CLI surface is cleaner and each command has one job
- Good, because JSON Schema is the contract format — standard, editable, portable
- Bad, because the repair strategies are lost (could be re-added as a separate `repair` command if needed)
