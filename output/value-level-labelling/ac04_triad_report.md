# ac-04 — corroboration triad (quarantine-first)

- scored value-rows: **25,609,271**
- KEEP: **17,967,246** (1,982,493 distinct values, 159 types)
- DROP (off-distribution): **7,447,260**
- FLAG/quarantine: **194,765** rows across **30,100** columns
- within-column heterogeneity removed (kept columns): **0.9%**
- categorical positive targets: **0** (excluded by policy)
- leakage collisions removed by row_hash firewall: **202099**
- latitude kept (distinct values): **10**

## Top kept types by distinct values

| type | kept rows | distinct values |
|---|---|---|
| representation.text.plain_text | 1827734 | 483719 |
| representation.numeric.integer_number | 9392959 | 466038 |
| representation.text.entity_name | 2455110 | 423565 |
| datetime.epoch.unix_seconds | 567538 | 133431 |
| representation.numeric.decimal_number | 378591 | 105199 |
| representation.identifier.numeric_code | 309616 | 61560 |
| identity.commerce.upc | 64937 | 59877 |
| geography.location.city | 453787 | 57556 |
| datetime.timestamp.sql_standard | 119792 | 50428 |
| representation.identifier.alphanumeric_id | 287273 | 34650 |
| identity.person.full_name | 290531 | 32179 |
| representation.identifier.uuid | 30085 | 19204 |
| geography.address.full_address | 64162 | 15210 |
| geography.location.region | 112598 | 14239 |
| datetime.epoch.unix_milliseconds | 15376 | 9819 |
| geography.location.country | 42709 | 9492 |
| datetime.timestamp.iso_8601 | 30852 | 8151 |
| datetime.date.mdy_slash | 47775 | 5428 |
| identity.commerce.isbn | 11857 | 4967 |
| datetime.date.compact_ymd | 43279 | 4894 |

Quarantine list: `output/value-level-labelling/quarantine_label_errors.csv` (30100 columns flagged as distillation label-errors — NOT auto-relabelled).
Cleaned training set: `output/value-level-labelling/cleaned_value_training.ndjson`.
