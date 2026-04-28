# Validate-Precision Corpus Report
Generated: 2026-04-28 19:21:10 +10:00
Threshold: P=0.99
Corpus: 7 datasets, 26119 rows total

## Headline
**3 of 7 datasets pass at P=99%** (baseline: 3 of 7; delta: +0)

## Per-mechanism breakdown
| Mechanism             | Failing columns | Datasets affected |
|-----------------------|-----------------|-------------------|
| enum_overfit          |               3 |                 2 |
| format_diversity      |               0 |                 0 |
| misclassification     |               7 |                 4 |
| code_vs_canonical     |               0 |                 0 |
| unknown               |               0 |                 0 |
| no_gt                 |               0 |                 0 |

## Per-dataset
| Dataset | Rows | Valid | Pass@99% | Failing columns | Top mechanism |
|---|---:|---:|:---:|---:|---|
| pokemon | 800 | 800 | ✓ | 0 | — |
| rio2016_athletes | 5000 | 0 | ✗ | 3 | enum_overfit |
| us_baby_names | 5000 | 0 | ✗ | 1 | misclassification |
| co2_emissions_by_nation | 5000 | 5000 | ✓ | 0 | — |
| world_population | 5000 | 0 | ✗ | 2 | misclassification |
| un_locode | 5000 | 0 | ✗ | 4 | misclassification |
| global_temp_annual | 319 | 319 | ✓ | 0 | — |

## Per-column attributions
| Dataset | Column | Mechanism |
|---|---|---|
| rio2016_athletes | id | misclassification |
| rio2016_athletes | nationality | enum_overfit |
| rio2016_athletes | sex | enum_overfit |
| us_baby_names | sex | misclassification |
| world_population | Country Code | enum_overfit |
| world_population | Value | misclassification |
| un_locode | Change | misclassification |
| un_locode | Function | misclassification |
| un_locode | Location | misclassification |
| un_locode | Subdivision | misclassification |

