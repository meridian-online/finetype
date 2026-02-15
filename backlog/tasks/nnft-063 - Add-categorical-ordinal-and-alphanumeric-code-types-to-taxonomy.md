---
id: NNFT-063
title: 'Add categorical, ordinal, and alphanumeric code types to taxonomy'
status: To Do
assignee: []
created_date: '2026-02-15 05:12'
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
- [ ] #1 representation.categorical type defined with generator producing small-set string values
- [ ] #2 representation.ordinal type defined with generator producing ordered discrete values
- [ ] #3 representation.code.alphanumeric_id type defined with generator producing mixed letter+digit strings
- [ ] #4 boolean relocated to representation.logical.boolean with technology.development.boolean as alias
- [ ] #5 All new types have validation patterns defined
- [ ] #6 Training data generated for each new type (≥800 samples each)
- [ ] #7 finetype check passes with all new definitions
<!-- AC:END -->
