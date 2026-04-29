# Validate-Precision Corpus Report
Generated: 2026-04-29 13:03:54 +10:00
Threshold: P=0.99
Corpus: 12 datasets, 46622 rows total
Mechanism reference: [docs/mechanism-attribution.md](../../docs/mechanism-attribution.md)

## Headline
**3 of 12 datasets pass at P=99%** (baseline: 3 of 7; delta: +0)

## Per-mechanism breakdown
| Mechanism             | Failing columns | Datasets affected |
|-----------------------|-----------------|-------------------|
| enum_overfit          |               6 |                 3 |
| format_diversity      |              16 |                 4 |
| misclassification     |              17 |                 6 |
| code_vs_canonical     |              39 |                 7 |
| unknown               |               0 |                 0 |
| no_gt                 |               0 |                 0 |

## Per-dataset
| Dataset | Rows | Valid | Pass@99% | Failing columns | Top mechanism |
|---|---:|---:|:---:|---:|---|
| pokemon | 800 | 800 | ✓ | 0 | — |
| rio2016_athletes | 5000 | 0 | ✗ | 3 | enum_overfit |
| us_baby_names | 5000 | 0 | ✗ | 1 | misclassification |
| co2_emissions_by_nation | 5000 | 5000 | ✓ | 0 | — |
| world_population | 5000 | 0 | ✗ | 2 | enum_overfit |
| un_locode | 5000 | 0 | ✗ | 4 | format_diversity |
| global_temp_annual | 319 | 319 | ✓ | 0 | — |
| sp500_constituents | 503 | 45 | ✗ | 2 | misclassification |
| nyc_taxi | 5000 | 0 | ✗ | 1 | misclassification |
| gdelt_events | 5000 | 0 | ✗ | 19 | format_diversity |
| fifa_players | 5000 | 0 | ✗ | 32 | code_vs_canonical |
| oecd_employment | 5000 | 0 | ✗ | 14 | misclassification |

## Per-column attributions
| Dataset | Column | Mechanism | Trigger |
|---|---|---|---|
| rio2016_athletes | id | code_vs_canonical | path-b-codetype |
| rio2016_athletes | nationality | enum_overfit | enum-constraint |
| rio2016_athletes | sex | enum_overfit | enum-constraint |
| us_baby_names | sex | misclassification | prediction-error |
| world_population | Country Code | enum_overfit | enum-constraint |
| world_population | Value | code_vs_canonical | path-b-codetype |
| un_locode | Change | format_diversity | path-b-prefix |
| un_locode | Function | format_diversity | path-b-prefix |
| un_locode | Location | code_vs_canonical | path-b-codetype |
| un_locode | Subdivision | format_diversity | path-b-prefix |
| sp500_constituents | Founded | misclassification | prediction-error |
| sp500_constituents | Symbol | code_vs_canonical | path-b-codetype |
| nyc_taxi | VendorID | misclassification | prediction-error |
| gdelt_events | ActionGeo_ADM1Code | format_diversity | path-b-prefix |
| gdelt_events | ActionGeo_CountryCode | enum_overfit | enum-constraint |
| gdelt_events | ActionGeo_FeatureID | format_diversity | path-b-prefix |
| gdelt_events | ActionGeo_FullName | misclassification | prediction-error |
| gdelt_events | Actor1Code | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor1Geo_ADM1Code | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor1Geo_CountryCode | enum_overfit | enum-constraint |
| gdelt_events | Actor1Geo_FeatureID | misclassification | prediction-error |
| gdelt_events | Actor1Geo_FullName | misclassification | prediction-error |
| gdelt_events | Actor1Type3Code | format_diversity | path-b-prefix |
| gdelt_events | Actor2Code | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor2Geo_ADM1Code | code_vs_canonical | path-b-codetype |
| gdelt_events | Actor2Geo_CountryCode | enum_overfit | enum-constraint |
| gdelt_events | Actor2Geo_FeatureID | format_diversity | path-b-prefix |
| gdelt_events | Actor2Geo_FullName | misclassification | prediction-error |
| gdelt_events | Actor2Religion2Code | format_diversity | path-b-prefix |
| gdelt_events | Actor2Type3Code | format_diversity | path-b-prefix |
| gdelt_events | FractionDate | misclassification | prediction-error |
| gdelt_events | SQLDATE | format_diversity | path-b-prefix |
| fifa_players | CAM | code_vs_canonical | path-b-codetype |
| fifa_players | CB | code_vs_canonical | path-b-codetype |
| fifa_players | CDM | code_vs_canonical | path-b-codetype |
| fifa_players | CF | code_vs_canonical | path-b-codetype |
| fifa_players | CM | code_vs_canonical | path-b-codetype |
| fifa_players | Contract Valid Until | format_diversity | path-b-prefix |
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
| fifa_players | Photo | misclassification | prediction-error |
| fifa_players | Preferred Foot | format_diversity | path-b-prefix |
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
| fifa_players | Value | code_vs_canonical | path-b-codetype |
| fifa_players | Wage | code_vs_canonical | path-b-codetype |
| oecd_employment | ACTION | format_diversity | path-b-prefix |
| oecd_employment | ACTIVITY | code_vs_canonical | path-b-codetype |
| oecd_employment | AGE | format_diversity | path-b-prefix |
| oecd_employment | Age | format_diversity | path-b-prefix |
| oecd_employment | Decimals | misclassification | prediction-error |
| oecd_employment | FREQ | misclassification | prediction-error |
| oecd_employment | Frequency of observation | format_diversity | path-b-prefix |
| oecd_employment | REF_AREA | misclassification | prediction-error |
| oecd_employment | SEX | misclassification | prediction-error |
| oecd_employment | STRUCTURE_ID | misclassification | prediction-error |
| oecd_employment | Sex | misclassification | prediction-error |
| oecd_employment | TIME_PERIOD | misclassification | prediction-error |
| oecd_employment | UNIT_MEASURE | code_vs_canonical | path-b-codetype |
| oecd_employment | UNIT_MULT | code_vs_canonical | path-b-codetype |

