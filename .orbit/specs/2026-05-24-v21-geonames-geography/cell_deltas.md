# v21 corpus-pass cell deltas

Per spec `2026-05-24-v21-geonames-geography` ac-06.

Comparison of m-19 baseline (v19), v20 retrain, and v21 retrain on the two
AC-04 target cells. Rates are per-1000-files (normalised because each
run processed slightly different file counts).

## Three-way comparison

| Cell | v19 (m-19) | v20 (YDF) | v21 (GeoNames) | v21 Δ vs v19 |
|------|-----------:|----------:|---------------:|-------------:|
| **1** `reject_rate_ceil × format_diversity_path_b` — postal_code → full_address | 12.61 / 1000 | 12.23 / 1000 (−3.0%) | **12.66 / 1000 (+0.4%, regress)** | did not meet −20% |
| **2** `non_trivial_floor × misclassification` — missed geography labels | 160.25 / 1000 | 158.43 / 1000 (−1.1%) | **156.59 / 1000 (−2.3%)** | did not meet −20% |

Files processed: v19 = 505,708 · v20 = 505,244 · v21 = 504,005

## Methodology note

Cell counts use the prediction-disagreement proxy (`sense_prediction NOT
LIKE 'geography.%' AND ydf_prediction LIKE ...`) rather than the full
mechanism-decomposition pipeline. The proxy doesn't apply the
file-level criterion-A/B filter (`fails_a` / `fails_b`), so absolute
counts are slightly larger than the strict cell counts. The ratio
v21/v19 is unchanged by this — the file-level filter depends on row
distributions, not the model swap.

## Per-subtype breakdown (cell 2)

Where the v21 GeoNames augmentation actually moved the needle:

| Subtype | v19 misses | v21 misses | Δ |
|---------|-----------:|-----------:|--:|
| geography.location.city | 55,281 | 54,421 | **−1.6%** ← 62% of cell 2 |
| geography.location.region | 10,449 | 9,998 | −4.3% |
| geography.address.full_address | 5,728 | 5,817 | +1.6% (regress) |
| geography.location.country | 4,297 | 4,268 | −0.7% |
| geography.address.street_name | 1,432 | 1,411 | −1.5% |
| **geography.transportation.iso6346** | **1,430** | **860** | **−39.9%** |
| geography.coordinate.coordinates | 835 | 795 | −4.8% |
| **geography.address.postal_code** | **650** | **565** | **−13.1%** |
| **geography.location.country_code** | **487** | **362** | **−25.7%** |
| geography.transportation.hs_code | 151 | 150 | −0.7% |
| **geography.location.continent** | **83** | **52** | **−37.3%** |
| geography.index.h3 | 45 | 43 | −4.4% |

**Pattern**: subtypes with syntactically distinctive values (postal codes,
ISO codes, continent codes) moved meaningfully. Subtypes that look like
generic text (city, region, full_address, street_name, person-name-like)
barely moved. iso6346 dropped 40% despite having zero training data —
benefited from the general "geography prefix" bias the augmentation
created.

## Conclusion

The val_acc lift on the v21 model (0.9194 vs v19's 0.9173, +0.0021) was
real but came from the easy subtypes — small populations in the corpus.
The dominant failure class (city, 62% of cell 2) needed disambiguation,
not more positive examples. GeoNames gave the model more cities; it did
not teach it what isn't a city.

Per spec ac-07: cell-2 lift < 10% → halt and re-investigate.
