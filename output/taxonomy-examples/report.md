# Taxonomy examples round-trip report

Engine: `finetype profile` (isolated single-column, full Sense+Sharpen). Header: leaf-name. Rows/column: 20. Binary: `/Users/hugh/github/meridian-online/finetype/target/release/finetype` (finetype 0.6.47).

**Round-trip: 233/249 (93.6%).**

A type *round-trips* when a column of its own taxonomy examples profiles back to that type. Failures are grouped below; not all are bugs — many are types the 244-dim model cannot predict and that the Sharpen recovery guards only fire for on real column context (membership thresholds, anchored patterns), which a small synthetic example column need not satisfy.

## Baseline diff

- Regressions (were passing, now fail): **0**
- New unacknowledged fails: **0**
- Graduated (acknowledged gap now passes): **3**

### Graduated (rerun --update-baseline to lock)
- `finance.securities.isin`
- `geography.address.street_name`
- `identity.person.password`

## All non-round-tripping types

| type | profiled as |
| --- | --- |
| `container.array.whitespace_separated` | `representation.text.plain_text` |
| `datetime.date.weekday_full_month` | `datetime.date.weekday_dmy_full` |
| `datetime.timestamp.rfc_2822_ordinal` | `datetime.timestamp.rfc_2822` |
| `finance.securities.cusip` | `representation.text.word` |
| `finance.securities.sedol` | `representation.identifier.numeric_code` |
| `geography.transportation.hs_code` | `representation.text.word` |
| `geography.transportation.unlocode` | `representation.text.word` |
| `identity.medical.cpt` | `representation.text.word` |
| `identity.medical.dea_number` | `representation.identifier.alphanumeric_id` |
| `identity.person.username` | `representation.text.word` |
| `representation.discrete.ordinal` | `representation.text.entity_name` |
| `representation.file.excel_format` | `representation.text.plain_text` |
| `representation.format.color_rgb` | `representation.text.word` |
| `representation.identifier.increment` | `representation.numeric.integer_number` |
| `representation.scientific.inchi` | `representation.text.plain_text` |
| `technology.code.imei` | `representation.numeric.integer_number` |
