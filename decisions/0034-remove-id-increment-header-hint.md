---
status: accepted
date-created: 2026-03-13
date-modified: 2026-03-13
---
# 0034. Remove id → increment hardcoded header hint

## Context and Problem Statement

The hardcoded header hint `"id" | "identifier" => increment` assumes all ID columns are numeric sequences (BIGINT). Real-world ID columns include UUIDs, alphanumeric codes (e.g., `us6000pgkh` in the USGS earthquake dataset), slugs, and sequential numbers. The hint overrides the model's correct value-level classification, causing `finetype load` to generate `CAST(id AS BIGINT)` which fails on non-numeric IDs.

## Considered Options

- Keep the hint and add a feature-based guard (check `is_numeric` ratio before applying)
- Remove the hint entirely and let the model decide based on values
- Replace with a weaker hint that maps to `alphanumeric_id` (VARCHAR)

## Decision Outcome

Chosen option: "Remove the hint entirely", because `id` columns are genuinely ambiguous — the model already classifies correctly without the hint (alphanumeric IDs → `categorical`, numeric IDs → `increment`). This is part of a broader investigation into replacing the hardcoded rule cascade with a learned disambiguator (see spike spec).

### Consequences

- Good, because the earthquake dataset `id` column is now classified correctly by the model
- Good, because it reduces the hardcoded rule surface area (75+ rules → 74)
- Bad, because numeric-only `id` columns may lose the `increment` classification (model may classify as `integer_number` instead — both are BIGINT, so load output is the same)
