# v23 vs v22 — gated cell-2 deltas

Per spec `2026-05-27-v23-precision-retrain` ac-04.

Cell-2 is the `sense NOT LIKE 'geography.%' AND ydf LIKE 'geography.%'` miss count under the gated YDF baseline (per spec `2026-05-26-ydf-validation-gate`). v23 must not regress vs v22 to satisfy ac-04's cell-2 component.

## Δ vs v19 (gated baseline)

| Variant | files | cell-2 gated | per-1k | Δ vs v19 | band |
|---|---:|---:|---:|---:|---|
| v19 | 505,708 | 77,874 | 153.99 | baseline | baseline |
| v22 | 503,643 | 69,458 | 137.91 | −10.4% | **Partial (10–20%)** |
| **v23** | 499,544 | 80,831 | 161.81 | **+5.1%** | Failed (< 10%) |

## Δ v22 → v23

- v22 gated cell-2 per-1k: 137.91
- v23 gated cell-2 per-1k: 161.81
- Δ v23 vs v22: **+17.3%**

## Per-subtype cell-2 trajectory

Where v23's geography accuracy held vs v22, and where it moved (positive Δ = regression, more geography misses).

| Subtype | v19 | v22 | v23 | Δ v22→v23 |
|---|---:|---:|---:|---:|
| **location.city** | 55,281 | 49,642 | 56,620 | **+6,978 (+14.1%)** |
| **location.region** | 10,449 | 9,110 | 11,752 | **+2,642 (+29.0%)** |
| address.full_address | 5,728 | 5,658 | 5,307 | −351 (−6.2%) |
| **location.country** | 4,297 | 2,945 | 5,016 | **+2,071 (+70.3%)** |
| address.street_name | 1,432 | 1,428 | 1,446 | +18 (+1.3%) |
| address.postal_code | 650 | 634 | 649 | +15 (+2.4%) |
| location.country_code | 11 | 11 | 10 | −1 (−9.1%) |
| transportation.unlocode | 7 | 7 | 7 | 0 (0.0%) |
| coordinate.longitude | 7 | 6 | 6 | 0 (0.0%) |
| transportation.iata_code | 6 | 11 | 12 | +1 (+9.1%) |
| location.continent | 2 | 2 | 2 | 0 (0.0%) |
| contact.calling_code | 2 | 2 | 2 | 0 (0.0%) |
| coordinate.latitude | 1 | 1 | 1 | 0 (0.0%) |
| transportation.icao_code | 1 | 1 | 1 | 0 (0.0%) |

## ac-04 band — cell-2 component

Pre-committed thresholds (relative to v22's −10.4% vs v19):
  - **Met** — v23 cell-2 vs v19 ≤ -10.4% (no regression)
  - **Partial** — within ±2pp of v22 (i.e. -8.4% to -12.4%)
  - **Failed** — > -8.4% (regresses ≥ 2pp vs v22)

v23 reads: **+5.1%** vs v19 gated.

**Verdict (cell-2 component): Failed**.

The final ac-04 band combines this with the FP-rate verdict (see `per_cluster_fp_rate.md`). The pair must both meet their threshold for the overall band:
  - Met overall: FP drop ≥ 50% AND cell-2 ≤ v22's −10.4%.
  - Partial overall: FP drop 20–50% AND cell-2 within ±2pp.
  - Failed overall: either component fails.
