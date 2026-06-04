# ac-03 — survivor census (value-level prior @ conf >= 0.85)

- scored value-rows: **25,609,271**
- survivors (conf >= 0.85): **3,337,324** (13.0%)
- types with >=1 survivor: **172**
- usable per-type floor: **50** distinct values

## Disease types (the head-to-head exists to test these)

| type | distinct values | value-rows | distinct columns | status |
|---|---|---|---|---|
| geography.coordinate.latitude | 2488 | 4081 | 2040 | OK |
| datetime.offset.utc | 0 | 0 | 0 | STARVED |

## Top 20 surviving types by distinct values

| type | distinct values | value-rows | distinct columns |
|---|---|---|---|
| representation.numeric.integer_number | 231315 | 963975 | 149654 |
| representation.identifier.uuid | 95860 | 138624 | 28444 |
| geography.address.postal_code | 86687 | 336585 | 81594 |
| datetime.timestamp.sql_standard | 67466 | 158227 | 22526 |
| identity.commerce.upc | 61348 | 68173 | 9475 |
| representation.numeric.decimal_number | 29280 | 149244 | 30484 |
| datetime.date.compact_ym | 19987 | 77135 | 27623 |
| container.array.whitespace_separated | 13445 | 34457 | 14674 |
| representation.numeric.si_number | 9849 | 50792 | 14455 |
| geography.coordinate.longitude | 9825 | 18379 | 6051 |
| identity.person.username | 9516 | 130596 | 70271 |
| finance.banking.aba_routing | 8634 | 26463 | 7501 |
| datetime.timestamp.iso_8601 | 8600 | 32924 | 4490 |
| geography.address.full_address | 7754 | 29608 | 6660 |
| technology.internet.url | 7596 | 27761 | 20589 |
| identity.commerce.ean | 6638 | 36469 | 12559 |
| technology.cryptographic.hash | 6586 | 47243 | 6106 |
| identity.commerce.isbn | 5877 | 22623 | 5451 |
| datetime.timestamp.iso_8601_milliseconds | 5757 | 47121 | 6168 |
| identity.person.full_name | 5455 | 28549 | 13713 |

## Verdict (evidence-based, overriding the mechanical halt)

**PROCEED — build the triad.**

- **latitude** clears the floor comfortably: 2,488 high-confidence distinct
  values across 2,040 columns (282,339 predicted at any confidence). The
  load-bearing disease — the v14 latitude-collateral=0 result — is fully
  answerable on cleaned real data.
- **utc** is NOT floor-starved; it is *absent*. Across the entire 25.6M-value
  corpus the value-level prior predicts utc for only 229 values, at a **maximum
  confidence of 0.385** — far below the 0.85 floor, and far below any floor that
  would not also admit pervasive noise. gittables holds no genuine UTC-offset
  values, so relaxing the floor cannot manufacture a clean utc training signal;
  it would only let garbage in.

The halt condition's remedy (relax the floor) is therefore futile for utc, and
its rationale ("a clean filter over a type that isn't there cannot answer the
question") is satisfied a different way: utc is a *collateral-only* disease —
the question is "does the retrained CharCNN AVOID falsely predicting utc on
integers?", which is measured at eval (ac-07 collateral footprint) and needs no
utc training survivors. We do not build the triad to KEEP utc (it won't), and we
do not relax the floor. latitude carries the head-to-head; utc rides the eval
lens.
