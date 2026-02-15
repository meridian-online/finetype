---
id: NNFT-067
title: Add column-name heuristic as soft inference signal
status: To Do
assignee: []
created_date: '2026-02-15 05:13'
labels:
  - feature
  - inference
dependencies: []
references:
  - crates/finetype-model/src/column.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Column names like "Age", "Name", "Email", "Gender" provide strong semantic signals about the data type. Currently column-mode inference ignores the column header entirely.

This is a bridge toward a full column embedding model. The approach:
- Maintain a mapping of common column name patterns to expected types (fuzzy matching)
- Use the column name as a soft signal that boosts or penalizes type candidates
- NOT a hard override — the name just adjusts confidence scores

Examples:
- Column named "age" or "Age" → boost identity.person.age, penalize technology.internet.port
- Column named "name" or "full_name" → boost identity.person.full_name
- Column named "email" → boost identity.person.email
- Column named "zip" or "postal" → boost geography.address.postal_code

This is a longer-term enhancement. Start with a simple keyword→type mapping and evaluate impact on GitTables benchmark before expanding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Column name keyword mapping covers at least 20 common column names
- [ ] #2 Column name signal adjusts confidence but never overrides a high-confidence model prediction
- [ ] #3 Titanic profiling improves on at least 3 columns when column names are used
- [ ] #4 GitTables column-mode evaluation shows no regression
- [ ] #5 Column name heuristic can be disabled via CLI flag (--no-header-hint)
- [ ] #6 Unit tests for column name matching and confidence adjustment
<!-- AC:END -->
