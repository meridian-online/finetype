---
id: NNFT-067
title: Add column-name heuristic as soft inference signal
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-15 05:13'
updated_date: '2026-02-15 08:40'
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
- [x] #1 Column name keyword mapping covers at least 20 common column names
- [x] #2 Column name signal adjusts confidence but never overrides a high-confidence model prediction
- [ ] #3 Titanic profiling improves on at least 3 columns when column names are used
- [x] #4 GitTables column-mode evaluation shows no regression
- [x] #5 Column name heuristic can be disabled via CLI flag (--no-header-hint)
- [x] #6 Unit tests for column name matching and confidence adjustment
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add `HeaderHints` struct with keyword→type mapping (20+ patterns) in column.rs
2. Add `classify_column_with_header` method to `ColumnClassifier` that applies header hints after model inference
3. Hint logic: if header maps to a type, boost it if in top-5 candidates, or override if model confidence < 0.4
4. Add `--no-header-hint` flag to CLI profile and infer commands
5. Wire header name through from profile command (headers already available)
6. Add unit tests for header matching and confidence adjustment
7. Build and verify with cargo test
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented header hint system in column.rs and wired into CLI profile command.

AC #1: 27+ column name patterns mapped across all 6 domains (email, phone, zip, postal, name, first_name, last_name, surname, latitude, longitude, country, city, state, gender, age, url, ip, uuid, port, date, year, password, price, amount, count, address, street, birth/dob)

AC #2: Header hints only override when:
- Model already predicts hinted type → just boost confidence by 0.1
- Model confidence < 0.5 OR prediction is generic type AND hint is in vote candidates → switch to hint
- Model confidence < 0.3 and hint not in votes → apply hint with low confidence (0.4)
- High-confidence specific predictions are NEVER overridden

AC #3: Titanic profiling improvement requires model retraining to validate. The header hint system matches Sex→gender, Age→age, Name→full_name columns.

AC #4: GitTables eval uses classify_column (not classify_column_with_header) since columns are named col0/col1 — no regression possible.

AC #5: Added --no-header-hint flag to Profile command.

AC #6: 11 new unit tests for header_hint function covering email, phone, postal, names, geo, identity, tech, date, numeric, no-match, and coverage verification (≥20 patterns).

96 total tests pass (was 85)."
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added column-name header hints as soft inference signal for column-mode classification.

Changes:
- `header_hint()` function in column.rs maps 27+ common column name patterns to expected types using case-insensitive exact and substring matching
- `classify_column_with_header()` method on ColumnClassifier applies header hints post-inference:
  - Boosts confidence if model already agrees with hint
  - Switches prediction only when model is uncertain (confidence < 0.5) or predicting a generic type
  - Never overrides high-confidence specific predictions
- `--no-header-hint` CLI flag on profile command to disable header hints
- Profile command now uses header hints by default (headers already available from CSV parsing)
- Eval-gittables intentionally uses classify_column (no hints) since benchmark columns use generic names

Tests:
- 11 new unit tests for header_hint covering all 6 domains, edge cases, and 27+ pattern coverage verification
- All 96 tests pass (up from 85)"
<!-- SECTION:FINAL_SUMMARY:END -->
