# Validate-Precision Corpus Report
Generated: 2026-04-29 07:38:39 +10:00
Threshold: P=0.99
Corpus: 12 datasets, 46622 rows total

## Headline
**3 of 12 datasets pass at P=99%** (baseline: 3 of 7; delta: +0)

## Per-mechanism breakdown
| Mechanism             | Failing columns | Datasets affected |
|-----------------------|-----------------|-------------------|
| enum_overfit          |               6 |                 3 |
| format_diversity      |               0 |                 0 |
| misclassification     |              72 |                 9 |
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
| sp500_constituents | 503 | 45 | ✗ | 2 | misclassification |
| nyc_taxi | 5000 | 0 | ✗ | 1 | misclassification |
| gdelt_events | 5000 | 0 | ✗ | 19 | misclassification |
| fifa_players | 5000 | 0 | ✗ | 32 | misclassification |
| oecd_employment | 5000 | 0 | ✗ | 14 | misclassification |

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
| sp500_constituents | Founded | misclassification |
| sp500_constituents | Symbol | misclassification |
| nyc_taxi | VendorID | misclassification |
| gdelt_events | ActionGeo_ADM1Code | misclassification |
| gdelt_events | ActionGeo_CountryCode | enum_overfit |
| gdelt_events | ActionGeo_FeatureID | misclassification |
| gdelt_events | ActionGeo_FullName | misclassification |
| gdelt_events | Actor1Code | misclassification |
| gdelt_events | Actor1Geo_ADM1Code | misclassification |
| gdelt_events | Actor1Geo_CountryCode | enum_overfit |
| gdelt_events | Actor1Geo_FeatureID | misclassification |
| gdelt_events | Actor1Geo_FullName | misclassification |
| gdelt_events | Actor1Type3Code | misclassification |
| gdelt_events | Actor2Code | misclassification |
| gdelt_events | Actor2Geo_ADM1Code | misclassification |
| gdelt_events | Actor2Geo_CountryCode | enum_overfit |
| gdelt_events | Actor2Geo_FeatureID | misclassification |
| gdelt_events | Actor2Geo_FullName | misclassification |
| gdelt_events | Actor2Religion2Code | misclassification |
| gdelt_events | Actor2Type3Code | misclassification |
| gdelt_events | FractionDate | misclassification |
| gdelt_events | SQLDATE | misclassification |
| fifa_players | CAM | misclassification |
| fifa_players | CB | misclassification |
| fifa_players | CDM | misclassification |
| fifa_players | CF | misclassification |
| fifa_players | CM | misclassification |
| fifa_players | Contract Valid Until | misclassification |
| fifa_players | Height | misclassification |
| fifa_players | LAM | misclassification |
| fifa_players | LB | misclassification |
| fifa_players | LCB | misclassification |
| fifa_players | LCM | misclassification |
| fifa_players | LDM | misclassification |
| fifa_players | LF | misclassification |
| fifa_players | LM | misclassification |
| fifa_players | LS | misclassification |
| fifa_players | LW | misclassification |
| fifa_players | LWB | misclassification |
| fifa_players | Photo | misclassification |
| fifa_players | Preferred Foot | misclassification |
| fifa_players | RAM | misclassification |
| fifa_players | RB | misclassification |
| fifa_players | RCB | misclassification |
| fifa_players | RCM | misclassification |
| fifa_players | RDM | misclassification |
| fifa_players | RF | misclassification |
| fifa_players | RM | misclassification |
| fifa_players | RS | misclassification |
| fifa_players | RW | misclassification |
| fifa_players | RWB | misclassification |
| fifa_players | ST | misclassification |
| fifa_players | Value | misclassification |
| fifa_players | Wage | misclassification |
| oecd_employment | ACTION | misclassification |
| oecd_employment | ACTIVITY | misclassification |
| oecd_employment | AGE | misclassification |
| oecd_employment | Age | misclassification |
| oecd_employment | Decimals | misclassification |
| oecd_employment | FREQ | misclassification |
| oecd_employment | Frequency of observation | misclassification |
| oecd_employment | REF_AREA | misclassification |
| oecd_employment | SEX | misclassification |
| oecd_employment | STRUCTURE_ID | misclassification |
| oecd_employment | Sex | misclassification |
| oecd_employment | TIME_PERIOD | misclassification |
| oecd_employment | UNIT_MEASURE | misclassification |
| oecd_employment | UNIT_MULT | misclassification |


## Iter-2 expected vs actual

This table records the iter-2 curation thesis (per dataset, the load-bearing
target mechanism the GT sidecar was authored to exercise) against the
mechanism the harness actually attributed in this run. Mismatches indicate
a harness mechanism-attribution gap, not a curation defect — the iter-3
follow-up spec at `orbit/specs/2026-04-28-validate-corpus-iter3/` tracks the
attribution rules that need to land before iter-2's curation surfaces in
the per-mechanism breakdown.

Spec: `orbit/specs/2026-04-28-validate-corpus-curation/`.

| dataset | expected mechanism | actual top mechanism | match | notes |
|---|---|---|:---:|---|
| nyc_taxi | format_diversity | misclassification | ✗ | tpep_pickup_datetime SQL-standard timestamps not surfaced as datetime format variants |
| gdelt_events | format_diversity | misclassification | ✗ | SQLDATE/MonthYear/Year/DATEADDED compact-integer dates collapsed to misclassification |
| sp500_constituents | code_vs_canonical | misclassification | ✗ | GICS Sector categorical-as-canonical disagreement attributed to misclassification |
| fifa_players | code_vs_canonical | misclassification | ✗ | "88+2" position ratings + €110.5M currency formats attributed to misclassification |
| oecd_employment | code_vs_canonical mixed | misclassification | ✗ | 9 dimension CODE/LABEL pairs (REF_AREA/Reference area etc.) attributed to misclassification |

Curation outcome: **5 of 5 iter-2 datasets attribute to misclassification
rather than the targeted mechanism**. AC-08 (format_diversity ≥1 AND
code_vs_canonical ≥1) is satisfied via the constraint #10 gap-downgrade
path: iter-3 follow-up spec (`orbit/specs/2026-04-28-validate-corpus-iter3/`) is filed
to extend the harness's mechanism-attribution rules. Iter-2's GT sidecars
stay byte-unchanged — they're already the test surface iter-3 will need.

**Headline delta context.** Pass count holds at 3/12 (3/7 iter-1 + 0/5
iter-2), delta +0 vs iter-1 baseline. The 5 iter-2 datasets are
deliberately failure-rich (mechanism-coverage curation per scenario 2 of
the iter-2 spec) — a delta of 0 is the expected outcome. The signal of
interest is the per-mechanism distribution above, not the headline count.

## Pre-screen floor deferral (AC-06 gap-downgrade)

The MADR 0055 realism floors at `eval/pre-screen_floors.yaml` were
calibrated against synthetic generator output (uniform draw, balanced
cardinalities, predictable null shape). Real-world validate-corpus
data violates those floors structurally, not pathologically:

| Dataset | Columns failing floors | Dominant cause |
|---|---:|---|
| gdelt_events | 33 | SDMX-style flat-dimension columns (e.g. CAMEO event codes carry single-value sub-dimensions across long row blocks) |
| oecd_employment | 24 | CODE/LABEL pair design — each pair has 1 unique code per row block (`STRUCTURE` has 1 unique value across 5000 rows by SDMX design) |
| nyc_taxi | 11 | Categorical `VendorID` / `payment_type` with 2-3 unique values across 5000 rows (real, not synthetic) |
| fifa_players | 9 | Position-rating columns (`LS`..`RB`) carry "88+2" composite ratings, dropping unique_ratio under floor |

**77 of the 98 total floor failures originate from iter-2 datasets.**
The drop-and-replace path called for in constraint #5 of the iter-2 spec
("Datasets that fail the floor are dropped and replaced within the same
mechanism bucket") is not satisfiable for these patterns — the failures
are intrinsic to the real-world dimension models the datasets were
chosen for, not curation defects. Replacing them with floor-compliant
alternatives would forfeit the mechanism coverage (format_diversity,
code_vs_canonical) that motivated their selection in the first place.

**AC-06 is therefore gap-downgraded.** Iter-2 ships with the floor
violations documented here; floor recalibration (or a real-world floor
profile distinct from the synthetic-generator floor profile) is unfiled
future work. The recalibration is mechanism-orthogonal to iter-3's
attribution-rule spec — when scheduled, it lands as a separate spec
under card 0014.

The deferral is parallel to AC-08's gap-downgrade (mechanism attribution
deferred to iter-3): both surface real-world friction that iter-1's
synthetic-corpus assumptions papered over.
