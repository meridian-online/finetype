# v23 precision retrain — hard-negative extraction

Per spec `2026-05-27-v23-precision-retrain` ac-01.

Hard negatives sourced from `eval/gittables/corpus_pass/columns.parquet` by filtering on the six `(sense_prediction, ydf_prediction)` pairs of the top-6 corroborated misclassification clusters. Dedup against `eval/row_hashes.tsv` per MADR 0056 — a column is excluded if ANY of its sample values, hashed with the shared normaliser, collides with the eval set.

## Per-cluster coverage

| cluster_id | sense FP label | YDF / correct label | pre-dedup | post-dedup | dropped |
|---|---|---|---:|---:|---:|
| `81b63a52e3ef…` | `rep.boolean.binary` | `rep.numeric.integer_number` | 37,268 | **37,251** | 17 |
| `721b890ea74d…` | `identity.person.gender_code` | `rep.discrete.categorical` | 24,028 | **24,028** | 0 |
| `1b858e0d073b…` | `datetime.offset.utc` | `rep.numeric.integer_number` | 23,158 | **23,121** | 37 |
| `cdde5d05b73a…` | `datetime.component.periodicity` | `rep.discrete.categorical` | 13,488 | **13,481** | 7 |
| `3f2aa8465552…` | `rep.identifier.alphanumeric_id` | `rep.discrete.categorical` | 12,748 | **12,734** | 14 |
| `20803deffbad…` | `technology.internet.url` | `rep.numeric.integer_number` | 3,686 | **3,686** | 0 |

## Totals

- Columns matching one of the six pairs: 114,376
- Excluded by eval row-hash collision: 75
- **Kept as hard negatives: 114,301**
- ac-01 minimum: 30,000 — PASS.

## Non-zero coverage check

**PASS** — all six clusters have ≥1 hard negative after dedup.
