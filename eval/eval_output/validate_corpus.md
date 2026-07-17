# Validate-Precision Corpus Report
Generated: 2026-07-17 17:46:24 +10:00
Threshold: P=0.99
Corpus: 12 datasets, 46622 rows total
Mechanism reference: [docs/mechanism-attribution.md](../../docs/mechanism-attribution.md)

## Headline
**5 of 12 datasets pass at P=99%** (baseline: 3 of 7; delta: +2)

## Per-mechanism breakdown
| Mechanism             | Failing columns | Datasets affected |
|-----------------------|-----------------|-------------------|
| enum_overfit          |               4 |                 2 |
| format_diversity      |              11 |                 4 |
| misclassification     |               8 |                 5 |
| code_vs_canonical     |              40 |                 4 |
| unknown               |               0 |                 0 |
| no_gt                 |               0 |                 0 |

## Per-dataset
| Dataset | Rows | Valid | Pass@99% | Failing columns | Top mechanism |
|---|---:|---:|:---:|---:|---|
| pokemon | 800 | 800 | ✓ | 0 | — |
| rio2016_athletes | 5000 | 4765 | ✗ | 1 | format_diversity |
| us_baby_names | 5000 | 5000 | ✓ | 0 | — |
| co2_emissions_by_nation | 5000 | 5000 | ✓ | 0 | — |
| world_population | 5000 | 0 | ✗ | 1 | enum_overfit |
| un_locode | 5000 | 0 | ✗ | 3 | code_vs_canonical |
| global_temp_annual | 319 | 319 | ✓ | 0 | — |
| sp500_constituents | 503 | 503 | ✓ | 0 | — |
| nyc_taxi | 5000 | 0 | ✗ | 1 | misclassification |
| gdelt_events | 5000 | 0 | ✗ | 20 | code_vs_canonical |
| fifa_players | 5000 | 0 | ✗ | 30 | code_vs_canonical |
| oecd_employment | 5000 | 0 | ✗ | 7 | format_diversity |

## Per-column attributions
| Dataset | Column | Mechanism | Trigger |
|---|---|---|---|
| rio2016_athletes | sport | format_diversity | path-b-prefix |
| world_population | Country Code | enum_overfit | enum-constraint |
| un_locode | Coordinates | misclassification | prediction-error |
| un_locode | Function | code_vs_canonical | path-b-codetype |
| un_locode | Location | code_vs_canonical | path-b-codetype |
| nyc_taxi | VendorID | misclassification | prediction-error |
| gdelt_events | ActionGeo_ADM1Code | code_vs_canonical | path-b-codetype |
| gdelt_events | ActionGeo_CountryCode | enum_overfit | enum-constraint |
| gdelt_events | ActionGeo_FeatureID | format_diversity | path-b-prefix |
| gdelt_events | ActionGeo_FullName | misclassification | prediction-error |
| gdelt_events | Actor1Code | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor1CountryCode | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor1Geo_CountryCode | enum_overfit | enum-constraint |
| gdelt_events | Actor1Geo_FullName | misclassification | prediction-error |
| gdelt_events | Actor1Name | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor1Type1Code | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor1Type2Code | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor2Code | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor2Geo_ADM1Code | format_diversity | path-b-prefix |
| gdelt_events | Actor2Geo_CountryCode | enum_overfit | enum-constraint |
| gdelt_events | Actor2Geo_FeatureID | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor2Geo_FullName | misclassification | prediction-error |
| gdelt_events | Actor2Name | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor2Type1Code | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor2Type2Code | code_vs_canonical | path-b-codetype |
| gdelt_events | FractionDate | format_diversity | path-b-prefix |
| fifa_players | Body Type | format_diversity | path-b-prefix |
| fifa_players | CAM | code_vs_canonical | path-b-codetype |
| fifa_players | CB | code_vs_canonical | path-b-codetype |
| fifa_players | CDM | code_vs_canonical | path-b-codetype |
| fifa_players | CF | code_vs_canonical | path-b-codetype |
| fifa_players | CM | code_vs_canonical | path-b-codetype |
| fifa_players | Height | misclassification | prediction-error |
| fifa_players | LAM | code_vs_canonical | path-b-codetype |
| fifa_players | LB | code_vs_canonical | path-b-codetype |
| fifa_players | LCB | code_vs_canonical | path-b-codetype |
| fifa_players | LCM | code_vs_canonical | path-b-codetype |
| fifa_players | LDM | code_vs_canonical | path-b-codetype |
| fifa_players | LF | code_vs_canonical | path-b-codetype |
| fifa_players | LM | code_vs_canonical | path-b-codetype |
| fifa_players | LS | code_vs_canonical | path-b-codetype |
| fifa_players | LW | code_vs_canonical | path-b-codetype |
| fifa_players | LWB | code_vs_canonical | path-b-codetype |
| fifa_players | RAM | code_vs_canonical | path-b-codetype |
| fifa_players | RB | code_vs_canonical | path-b-codetype |
| fifa_players | RCB | code_vs_canonical | path-b-codetype |
| fifa_players | RCM | code_vs_canonical | path-b-codetype |
| fifa_players | RDM | code_vs_canonical | path-b-codetype |
| fifa_players | RF | code_vs_canonical | path-b-codetype |
| fifa_players | RM | code_vs_canonical | path-b-codetype |
| fifa_players | RS | code_vs_canonical | path-b-codetype |
| fifa_players | RW | code_vs_canonical | path-b-codetype |
| fifa_players | RWB | code_vs_canonical | path-b-codetype |
| fifa_players | ST | code_vs_canonical | path-b-codetype |
| fifa_players | Value | format_diversity | path-b-prefix |
| fifa_players | Wage | format_diversity | path-b-prefix |
| oecd_employment | AGE | format_diversity | path-b-prefix |
| oecd_employment | SEX | format_diversity | path-b-prefix |
| oecd_employment | STRUCTURE_ID | format_diversity | path-a-pattern |
| oecd_employment | STRUCTURE_NAME | misclassification | prediction-error |
| oecd_employment | TIME_PERIOD | misclassification | prediction-error |
| oecd_employment | TRANSFORMATION | format_diversity | path-b-prefix |
| oecd_employment | UNIT_MEASURE | code_vs_canonical | path-b-codetype |

