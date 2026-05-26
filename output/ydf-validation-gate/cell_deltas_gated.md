# Gated cell-2 — v19/v20/v21/v22 against the cleaned baseline

Per spec `2026-05-26-ydf-validation-gate` ac-04.

Cell-2 (`sense NOT LIKE 'geography.%' AND ydf LIKE 'geography.%'`) recomputed using `ydf_prediction_gated` instead of raw `ydf_prediction`. Any YDF prediction the gate refused (per ac-01) drops out — the metric stops penalising Sense for disagreeing with demonstrably-wrong YDF labels (msg_id as iso6346, stock_id as mgrs, team-codes as country_code, ...).

## Raw vs gated cell-2

| Variant | files | cell-2 raw | cell-2 gated | drop | raw per-1k | gated per-1k |
|---|---:|---:|---:|---:|---:|---:|
| v19 | 505,708 | 81,037 | **77,874** | -3,163 (-3.9%) | 160.24 | **153.99** |
| v20 | 505,244 | 80,044 | **77,610** | -2,434 (-3.0%) | 158.43 | **153.61** |
| v21 | 504,005 | 78,922 | **76,513** | -2,409 (-3.1%) | 156.59 | **151.81** |
| v22 | 503,643 | 73,514 | **69,458** | -4,056 (-5.5%) | 145.96 | **137.91** |

## Δ vs v19 (gated baseline)

| Variant | gated per-1k | Δ vs v19 gated | band |
|---|---:|---:|---|
| v19 | 153.99 | — | baseline |
| v20 | 153.61 | −0.2% | Failed (< 10%) |
| v21 | 151.81 | −1.4% | Failed (< 10%) |
| **v22** | 137.91 | **−10.4%** | **Partial (10–20%)** |

## Per-subtype cell-2 — v19 → v22 (gated)

Where v22 actually moved the needle once the metric stops scoring against demonstrably-wrong YDF labels.

| Subtype | v19 gated | v22 gated | Δ |
|---|---:|---:|---:|
| location.city | 55,281 | 49,642 | −5,639 (−10.2%) |
| location.region | 10,449 | 9,110 | −1,339 (−12.8%) |
| address.full_address | 5,728 | 5,658 | −70 (−1.2%) |
| **location.country** | 4,297 | 2,945 | **−1,352 (−31.5%)** |
| address.street_name | 1,432 | 1,428 | −4 (−0.3%) |
| address.postal_code | 650 | 634 | −16 (−2.5%) |
| **transportation.iata_code** | 6 | 11 | **+5 (+83.3%)** |
| location.country_code | 11 | 11 | 0 (0.0%) |
| coordinate.longitude | 7 | 6 | −1 (−14.3%) |
| transportation.unlocode | 7 | 7 | 0 (0.0%) |
| location.continent | 2 | 2 | 0 (0.0%) |
| contact.calling_code | 2 | 2 | 0 (0.0%) |
| coordinate.latitude | 1 | 1 | 0 (0.0%) |
| transportation.icao_code | 1 | 1 | 0 (0.0%) |

## Compare to noisy baseline (was v22 in band?)

- v22 vs v19 on the **noisy** baseline: -8.9% (per output/corpus-pass-v22/cell_deltas.md — Failed band).
- v22 vs v19 on the **gated** baseline: -10.4%.

**v22 reaches the Partial band against an honest baseline.**
