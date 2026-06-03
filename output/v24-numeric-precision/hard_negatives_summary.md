# v24 numeric-precision — hard-negative extraction

Per spec `2026-06-03-v24-numeric-precision` ac-01. Numeric-target clusters ONLY; every categorical-target cluster is excluded by construction (the v23 failure mode).

## Per-cluster coverage

| cluster_id | sense FP label | ydf correct | safety | pre-dedup | kept | dropped |
|---|---|---|---:|---:|---:|---:|
| `bool_to_int` | `representation.boolean.binary` | `rep.numeric.integer_number` | 0.94 | 37,268 | **37,251** | 17 |
| `utc_to_int` | `datetime.offset.utc` | `rep.numeric.integer_number` | 0.95 | 23,158 | **23,121** | 37 |
| `int_to_dec` | `representation.numeric.integer_number` | `rep.numeric.decimal_number` | 0.84 | 14,556 | **14,554** | 2 |
| `url_to_int` | `technology.internet.url` | `rep.numeric.integer_number` | 0.91 | 3,686 | **3,686** | 0 |

## Gates

- Safety floor 0.8: all clusters pass
- Leakage dedup (MADR 0056): 56 columns dropped on eval row-hash collision.
- Non-zero coverage on all four: **PASS**
- correct_label set = `['representation.numeric.decimal_number', 'representation.numeric.integer_number']` — zero categorical: **PASS** (asserted).

## Total kept: 78,612
