---
status: accepted
date-created: 2026-03-03
date-modified: 2026-03-11
---
# 0008. format_string_alt for ISO 8601 variants

## Context and Problem Statement

FineType's actionability eval (NNFT-191) revealed that certain datetime types have legitimate format variants that a single `format_string` cannot capture. The primary case: ISO 8601 timestamps may include fractional seconds (`%Y-%m-%dT%H:%M:%S%.f`) or not (`%Y-%m-%dT%H:%M:%S`). A column classified as `iso_8601` could contain either variant, but `TRY_STRPTIME` with the wrong format string returns NULL.

Actionability dropped to 92.7% partly because the eval framework only tried one format string per type.

## Considered Options

- **Option A — Multiple format_string fields (format_string_1, format_string_2, ...).** Ugly, rigid, doesn't scale.
- **Option B — format_string_alt as a single alternative.** Add one `format_string_alt` field to the Definition struct. The eval framework (and downstream tools) try `format_string` first, then `format_string_alt` if the first fails. Covers the dominant case (with/without fractional seconds) without over-engineering.
- **Option C — Array of format strings.** Most flexible but changes the YAML schema and every consumer.

## Decision Outcome

Chosen option: **Option B — format_string_alt**, because it solves the immediate problem (ISO 8601 ±fractional seconds) with minimal schema change. The YAML `format_string_alt` field is optional and only populated for types with a known secondary format.

Actionability improved from 92.7% → 96.0% after this change plus eval framework updates to try both formats.

### Consequences

- Good, because actionability improved by 3.3pp with a simple schema addition
- Good, because the Definition struct change is backward compatible (Option field)
- Bad, because it only supports one alternative — types with 3+ legitimate formats would need a different approach
- Neutral, because the field is currently only used by the eval framework, not by `finetype profile` output — but it could be exposed in schema exports if needed
