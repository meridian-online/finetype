---
status: accepted
date-created: 2026-03-12
date-modified: 2026-03-12
---
# 0033. Sidecar file output contract for validate

## Context and Problem Statement

qsv issue #1728 demonstrated that conditional file creation breaks CI/CD pipelines — when all rows are valid, qsv didn't create a `.valid` file, forcing users to add branching logic. DuckDB's rejects tables always exist. The validate command needs predictable output for automation.

## Considered Options

- Sidecar files only when errors exist (qsv original behaviour)
- Sidecar files always, no opt-out
- Sidecar files always by default, `--summary-only` to suppress

## Decision Outcome

Chosen option: "`--summary-only` to suppress", because always-write is the safe default for automation (no conditional file creation), but forced file creation annoys interactive users and breaks read-only contexts. `--summary-only` gives explicit control.

### Consequences

- Good, because pipelines can rely on `.valid.csv`, `.invalid.csv`, `.errors.jsonl` always existing
- Good, because interactive users can use `--summary-only` for quick checks
- Good, because the all-valid case produces predictable output (`.invalid.csv` = headers only, `.errors.jsonl` = empty)
