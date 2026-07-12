# Taxonomy examples round-trip report

Engine: `finetype profile` (isolated single-column, full Sense+Sharpen). Header: leaf-name. Rows/column: 20. Binary: `target/release/finetype` (finetype 0.6.47).

**Round-trip: 241/249 (96.8%).**

A type *round-trips* when a column of its own taxonomy examples profiles back to that type. Failures are grouped below; not all are bugs — many are types the 244-dim model cannot predict and that the Sharpen recovery guards only fire for on real column context (membership thresholds, anchored patterns), which a small synthetic example column need not satisfy.

## Baseline diff

- Regressions (were passing, now fail): **0**
- New unacknowledged fails: **0**
- Graduated (acknowledged gap now passes): **0**

## All non-round-tripping types

| type | profiled as |
| --- | --- |
| `container.array.whitespace_separated` | `representation.text.plain_text` |
| `datetime.date.weekday_full_month` | `datetime.date.weekday_dmy_full` |
| `datetime.timestamp.rfc_2822_ordinal` | `datetime.timestamp.rfc_2822` |
| `identity.person.username` | `representation.text.word` |
| `representation.discrete.ordinal` | `representation.text.entity_name` |
| `representation.file.excel_format` | `representation.text.plain_text` |
| `representation.identifier.increment` | `representation.numeric.integer_number` |
| `representation.scientific.inchi` | `representation.text.plain_text` |
