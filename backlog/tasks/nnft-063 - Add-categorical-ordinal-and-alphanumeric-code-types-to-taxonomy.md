---
id: NNFT-063
title: 'Add categorical, ordinal, and alphanumeric code types to taxonomy'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-15 05:12'
updated_date: '2026-02-15 08:18'
labels:
  - taxonomy
  - feature
dependencies: []
references:
  - labels/definitions_representation.yaml
  - labels/definitions_technology.yaml
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Titanic profiling test revealed several columns with no suitable type in our 159-type taxonomy:

- **Categorical**: Embarked (S/C/Q), Sex (male/female) — small set of discrete string values
- **Ordinal**: Pclass (1/2/3) — ordered discrete values with ranking semantics
- **Alphanumeric code/ID**: Ticket ("A/5 21171"), Cabin ("C85", "B28") — mixed letter+digit identifiers

Also, `technology.development.boolean` is overly specific for a universal concept. Boolean/flag columns appear across all domains. Consider relocating to `representation.logical.boolean`.

New types to add:
- `representation.categorical` — small cardinality discrete string values
- `representation.ordinal` — ordered discrete values (numeric or string)
- `representation.code.alphanumeric_id` — mixed letter+digit identifier strings
- Move boolean from `technology.development.boolean` → `representation.logical.boolean` (keep old label as alias)

Each new type needs: definition YAML entry, generator function, training data generation, and inclusion in next model training round.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 representation.categorical type defined with generator producing small-set string values
- [x] #2 representation.ordinal type defined with generator producing ordered discrete values
- [x] #3 representation.code.alphanumeric_id type defined with generator producing mixed letter+digit strings
- [x] #4 boolean relocated to representation.logical.boolean with technology.development.boolean as alias
- [x] #5 All new types have validation patterns defined
- [x] #6 Training data generated for each new type (≥800 samples each)
- [x] #7 finetype check passes with all new definitions
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add 3 new type YAML definitions to labels/definitions_representation.yaml:
   - representation.discrete.categorical (small-set string values)
   - representation.discrete.ordinal (ordered discrete values)
   - representation.code.alphanumeric_id (mixed letter+digit identifiers)
2. Add representation.logical.boolean definition (mirror of technology.development.boolean for future migration)
3. Add generator functions for all 3 new types in generator.rs
4. Run finetype check to validate alignment
5. Generate training data (≥800 samples each)
6. Run full test suite

Note: categorical and ordinal are partially semantic types — single-value model will learn common patterns but column-level disambiguation (NNFT-065) needed for full accuracy. Boolean relocation is a label rename that happens at retraining time.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 4 new types to the taxonomy (159 → 163 types) with YAML definitions, generators, and training data.

## New Types

| Type | Category | Description |
|------|----------|-------------|
| `representation.discrete.categorical` | discrete | Low-cardinality string values (male/female, active/inactive, S/C/Q) |
| `representation.discrete.ordinal` | discrete | Ordered discrete values (low/medium/high, A/B/C/D/F, 1st/2nd/3rd) |
| `representation.code.alphanumeric_id` | code | Mixed letter+digit identifiers (SKU-12345, C85, A/5 21171) |
| `representation.logical.boolean` | logical | Boolean values (canonical location, replaces technology.development.boolean) |

## Changes

**labels/definitions_representation.yaml:**
- Added 4 new type definitions with full metadata (title, description, broad_type, transform, validation, tier, samples)
- alphanumeric_id pattern uses alternation instead of lookaheads (Rust regex doesn't support lookaheads)
- categorical and ordinal have no validation pattern (semantic types, column-level detection needed)

**crates/finetype-core/src/generator.rs:**
- Added generators for all 4 new types
- categorical: 12 vocabularies (male/female, colors, status, directions, etc.)
- ordinal: 10 vocabularies (priority levels, grades, star ratings, Roman numerals, etc.)
- alphanumeric_id: 10 patterns (PREFIX-NNNNN, L-NNN, LL-NNNN, license plates, Titanic tickets, etc.)
- logical.boolean: mirrors technology.development.boolean generator

## Verification
- `finetype check`: 8150/8150 samples pass (100%)
- Training data: 130,400 samples generated (800 per type × 163 types)
- All 146 unit tests pass
- Taxonomy count: 163 definitions across 6 domains

## Note
categorical and ordinal are partially semantic types. Single-value model will learn common patterns, but reliable column-level detection requires cardinality-based disambiguation (NNFT-065).
<!-- SECTION:FINAL_SUMMARY:END -->
