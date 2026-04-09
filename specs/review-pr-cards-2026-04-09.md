# Pre-Merge Review

**Date:** 2026-04-09
**Reviewer:** Context-separated agent (fresh session)
**Branch:** cards/capability-distillation
**Spec:** N/A (distillation, not spec-driven)
**Verdict:** APPROVE

## Validation Results

```
| Check                    | Result | Details                                                                                          |
|--------------------------|--------|--------------------------------------------------------------------------------------------------|
| YAML syntax              | PASS   | All 12 files parse cleanly with PyYAML                                                           |
| Schema compliance        | PASS   | All 12 cards have required fields (feature, as_a, i_want, so_that, scenarios, maturity, goal, specs, references). Every scenario has name/given/when/then/source_lines |
| No stale priority field  | PASS   | Zero occurrences of `priority` across all cards                                                  |
| No hardcoded counts      | WARN   | "84 datetime types" in 0001 then field (non-source_lines); see finding below                     |
| Sequential numbering     | PASS   | 0001-0012, no gaps                                                                               |
| Source traceability       | WARN   | 2 source files are web-only (agent-ready.mdx, schema.mdx); 1 scenario uses "user-requested during edit"; see findings below |
| Maturity values          | PASS   | 9 established, 3 emerging (0005, 0011, 0012) -- all valid values                                |
| Spec references          | PASS   | All 11 spec directory references resolve to existing paths                                       |
| Reference files          | PASS   | All local references exist; 2 use `web:` prefix (correctly flagged as external)                  |
| Overlap check            | PASS   | No two cards describe the same capability; see analysis below                                    |
```

## Findings

### [LOW] Hardcoded "84" in 0001 then field

`cards/0001-type-taxonomy.yaml` line 10:
```yaml
then: "all 84 datetime types are listed with their categories and descriptions"
```

This is a hardcoded count in a `then` field (not `source_lines`). The distillation deliberately avoided hardcoded counts in non-quote fields, but this one slipped through. The number matches both README and CLAUDE.md today (both say datetime: 84), so it is not currently wrong -- but it will go stale the next time a datetime type is added or removed.

**Suggestion:** Replace with "all datetime types are listed with their categories and descriptions" to match the staleness-avoidance pattern used elsewhere.

### [LOW] 250 vs 239 inconsistency in 0001 source_lines vs goal

`cards/0001-type-taxonomy.yaml` has two counts that disagree:
- Line 29 `source_lines`: quotes README verbatim -- "FineType recognizes 250 types across 7 domains" (README is accurate: 84+36+28+34+31+25+12 = 250)
- Line 32 `goal`: says "239 types across 7 domains" (CLAUDE.md says 239 definitions with different subcounts: 11+84+28+24+33+33+26 = 239)

The README and CLAUDE.md themselves are out of sync (250 vs 239). The card faithfully quotes each source, so the card is not wrong per se -- the upstream documents disagree. This is not a card defect but worth noting: the README likely includes locale-specific subtypes in its count while CLAUDE.md counts base definitions.

**Suggestion:** No card change needed. The README/CLAUDE.md count discrepancy is a separate housekeeping item.

### [INFO] Non-local source_lines references

Two cards reference files that exist only on the Meridian web site, not in this repo:

- `cards/0011-agent-ready-skills.yaml`: all 3 scenarios cite `agent-ready.mdx`
- `cards/0012-pii-detection.yaml`: all 3 scenarios cite `schema.mdx`

These are correctly marked with `web:` prefix in the `references` field, but the `source_lines` values use bare filenames (`agent-ready.mdx`, `schema.mdx`) without the `web:` prefix. This is cosmetically inconsistent but not a schema violation -- `source_lines` is a free-text attribution, not a path reference.

### [INFO] One scenario uses non-file source attribution

`cards/0005-schema-driven-data-validation.yaml` scenario "Separate valid from invalid with SQL" has:
```yaml
source_lines: "user-requested during edit"
```

This is a valid attribution (the scenario was added during the distillation session at Hugh's request), but it breaks the pattern of `filename: quote` used everywhere else. Acceptable for a distillation artifact.

## Overlap Analysis

The 12 cards partition FineType's capabilities cleanly:

- **Core engine:** 0001 (taxonomy/contracts), 0002 (inference/classification), 0009 (locale detection)
- **User workflows:** 0003 (profiling), 0004 (loading), 0005 (validation), 0010 (generation)
- **Interfaces:** 0006 (CLI), 0007 (DuckDB extension), 0008 (MCP server), 0011 (agent skills)
- **Cross-cutting:** 0012 (PII detection)

The closest pair is 0005 (schema-driven validation) and 0007 (DuckDB extension), which both mention `finetype_validate`. This is complementary, not duplicative: 0005 describes the validation *workflow* (generate schema, validate, filter), while 0007 describes the DuckDB *interface* (6 scalar functions including validate). No merge needed.

## Honest Assessment

This is a clean, well-structured set of capability cards. The orbit v0.2.10 schema is fully satisfied across all 12 files with zero syntax errors, zero missing fields, and zero stale `priority` fields. The only actionable finding is the hardcoded "84" in card 0001's `then` field, which is a low-severity staleness risk. The 250-vs-239 count discrepancy originates in the upstream documentation (README vs CLAUDE.md), not in the cards themselves. The cards partition FineType's capabilities into distinct, non-overlapping domains with good source traceability. This PR is ready to merge.
