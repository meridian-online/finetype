# v22 + v23 (R26 Sharpen) corpus-pass cell deltas

Per spec `2026-05-26-v23-sharpen-code-discriminator` ac-07.

Five-way comparison: v19 baseline, v20/v21/v22 retrains, and v22+v23 where v23's R26 country_code Sharpen rule is applied offline to v22's corpus pass (no fresh corpus pass — pure post-Sense post-processing).

## Five-way comparison (cell-2)

| Variant | files | cell-2 | per-1k | Δ vs v19 |
|---|---:|---:|---:|---:|
| v19 | 505,708 | 81,037 | 160.24 | — |
| v20 | 505,244 | 80,044 | 158.43 | −1.1% |
| v21 | 504,005 | 78,922 | 156.59 | −2.3% |
| v22 | 503,643 | 73,514 | 145.96 | −8.9% |
| **v22+v23** | 503,643 | 73,513 | 145.96 | **−8.9%** |

**R26 incremental lift vs v22-Sense-only:** -1 columns moved out of cell-2 (-0.00% relative).

## Per-subtype cell-2 (v22 → v22+v23)

R26 only promotes to country_code; every other subtype should be unchanged unless a column whose Sense label changed happened to also flip the cell-2 inclusion check.

| Subtype | v22 misses | v22+v23 misses | Δ |
|---|---:|---:|---:|
| location.city | 49,642 | 49,642 | 0 |
| location.region | 9,110 | 9,109 | −1 |
| address.full_address | 5,658 | 5,658 | 0 |
| location.country | 2,945 | 2,945 | 0 |
| transportation.iso6346 | 2,007 | 2,007 | 0 |
| address.street_name | 1,428 | 1,428 | 0 |
| **location.country_code** | 948 | 948 | **0** |
| coordinate.coordinates | 739 | 739 | 0 |
| address.postal_code | 634 | 634 | 0 |
| transportation.hs_code | 159 | 159 | 0 |
| coordinate.geohash | 52 | 52 | 0 |
| index.h3 | 44 | 44 | 0 |
| coordinate.mgrs | 35 | 35 | 0 |
| contact.calling_code | 31 | 31 | 0 |
| transportation.unlocode | 23 | 23 | 0 |
| location.continent | 21 | 21 | 0 |
| transportation.iata_code | 11 | 11 | 0 |
| coordinate.longitude | 11 | 11 | 0 |
| coordinate.latitude | 6 | 6 | 0 |
| coordinate.plus_code | 4 | 4 | 0 |
| transportation.icao_code | 4 | 4 | 0 |
| format.wkt | 2 | 2 | 0 |

## ac-08 band — R26-scoped

v22+v23 cell-2 rate: **145.96 / 1000** (vs v19 160.24, Δ -8.9%).

Band: **Lift ≥ 0 (per AC-08 R26-scoped band)** — R26 fires productively; country_code recovery works as designed.
