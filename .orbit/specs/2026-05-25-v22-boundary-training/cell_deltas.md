# v22 corpus-pass cell deltas

Per spec `2026-05-25-v22-boundary-training` ac-07.

Comparison of m-19 baseline (v19), v20 retrain, v21 retrain, and v22 retrain on the two AC-04 target cells. Rates are per-1000-files (normalised because each run processed slightly different file counts).

## Four-way comparison

| Cell | v19 (m-19) | v20 (YDF) | v21 (GeoNames) | v22 (boundary) | v22 Δ vs v19 |
|------|-----------:|----------:|---------------:|---------------:|-------------:|
| **1** `reject_rate_ceil × format_diversity_path_b` — postal_code → full_address | 12.32 / 1000 | 12.12 / 1000 (−1.6%) | 12.45 / 1000 (+1.1%) | **12.32 / 1000 (+0.0%)** | did not meet −20% |
| **2** `non_trivial_floor × misclassification` — missed geography labels | 160.24 / 1000 | 158.43 / 1000 (−1.1%) | 156.59 / 1000 (−2.3%) | **145.96 / 1000 (−8.9%)** | did not meet −20% |

Files processed: v19 = 505,708 · v20 = 505,244 · v21 = 504,005 · v22 = 503,643

## Methodology note

Cell counts use the prediction-disagreement proxy (`sense_prediction NOT LIKE 'geography.%' AND ydf_prediction LIKE 'geography.%'` for Cell 2; `ydf_prediction = 'geography.address.full_address' AND sense_prediction disagrees` for Cell 1) rather than the full mechanism-decomposition pipeline. Cell 2's proxy reproduces v21's published v19 baseline (160.25/1000) exactly. Cell 1's proxy lands ~2.4% below v21's published 12.61/1000 — the proxy doesn't apply the file-level criterion-B filter so absolute counts differ slightly. The ratio v22/v19 is unchanged by this — the file-level filter depends on row distributions, not the model swap.

## Per-subtype breakdown (cell 2)

Where the v22 boundary training actually moved the needle (per-subtype miss counts; v19 → v22 percent change):

| Subtype | v19 misses | v21 misses | v22 misses | v22 Δ vs v19 |
|---------|-----------:|-----------:|-----------:|-------------:|
| location.city | 55,281 | 54,421 | 49,642 | −10.2% |
| location.region | 10,449 | 9,998 | 9,110 | −12.8% |
| address.full_address | 5,728 | 5,817 | 5,658 | −1.2% |
| **location.country** | 4,297 | 4,268 | 2,945 | **−31.5%** |
| **transportation.iso6346** | 1,430 | 860 | 2,007 | **+40.3%** |
| address.street_name | 1,432 | 1,411 | 1,428 | −0.3% |
| **location.country_code** | 487 | 362 | 948 | **+94.7%** |
| coordinate.coordinates | 835 | 795 | 739 | −11.5% |
| address.postal_code | 650 | 565 | 634 | −2.5% |
| transportation.hs_code | 151 | 150 | 159 | +5.3% |
| **location.continent** | 83 | 52 | 21 | **−74.7%** |
| coordinate.geohash | 44 | 46 | 52 | +18.2% |
| index.h3 | 45 | 43 | 44 | −2.2% |
| **coordinate.mgrs** | 20 | 38 | 35 | **+75.0%** |
| contact.calling_code | 29 | 27 | 31 | +6.9% |
| **format.wkt** | 25 | 6 | 2 | **−92.0%** |
| transportation.unlocode | 20 | 30 | 23 | +15.0% |
| coordinate.longitude | 12 | 11 | 11 | −8.3% |
| **transportation.iata_code** | 6 | 8 | 11 | **+83.3%** |
| coordinate.latitude | 6 | 6 | 6 | +0.0% |
| transportation.icao_code | 4 | 4 | 4 | +0.0% |
| **coordinate.plus_code** | 3 | 4 | 4 | **+33.3%** |

## ac-08 band — cell-2 lift vs v19

v22 cell-2 rate: **145.96 / 1000** (vs v19 160.24). Δ = -8.9%.

Band: **Failed (< 10% reduction)** — training-data interventions exhausted; architectural surgery next.
