# v22 direction review — per-subtype trajectory

Per spec `2026-05-26-v22-gated-direction-review` ac-01.

Cell-2 miss counts per geography subtype, version-by-version, scored against the gated YDF baseline (per spec `2026-05-26-ydf-validation-gate`). Sorted by v19 miss count descending — the dominant miss classes surface first.

Bands (rightmost column): **monotone-mover** = v19≥v20≥v21≥v22 with ≥10% v19→v22 drop; **v22-jumper** = flat v19→v21 then ≥10% v21→v22 drop; **flat** = |v19→v22| < 5%; **regressed** = v22 worse by ≥5%; **mixed** = none of the above.

## Files in each corpus pass

| version | files |
|---|---:|
| v19 | 505,708 |
| v20 | 505,244 |
| v21 | 504,005 |
| v22 | 503,643 |

## Per-subtype trajectory

| Subtype | v19 | v20 | v21 | v22 | Δ v19→v22 | Δ v21→v22 | band |
|---|---:|---:|---:|---:|---:|---:|---|
| **location.city** | 55,281 | 55,385 | 54,421 | 49,642 | −5,639 (−10.2%) | −4,779 (−8.8%) | **monotone-mover** |
| **location.region** | 10,449 | 10,231 | 9,998 | 9,110 | −1,339 (−12.8%) | −888 (−8.9%) | **monotone-mover** |
| address.full_address | 5,728 | 5,585 | 5,817 | 5,658 | −70 (−1.2%) | −159 (−2.7%) | flat |
| **location.country** | 4,297 | 4,362 | 4,268 | 2,945 | −1,352 (−31.5%) | −1,323 (−31.0%) | **monotone-mover** |
| address.street_name | 1,432 | 1,418 | 1,411 | 1,428 | −4 (−0.3%) | +17 (+1.2%) | flat |
| address.postal_code | 650 | 596 | 565 | 634 | −16 (−2.5%) | +69 (+12.2%) | flat |
| location.country_code | 11 | 11 | 6 | 11 | 0 (0.0%) | +5 (+83.3%) | flat |
| transportation.unlocode | 7 | 7 | 7 | 7 | 0 (0.0%) | 0 (0.0%) | flat |
| **coordinate.longitude** | 7 | 2 | 6 | 6 | −1 (−14.3%) | 0 (0.0%) | **monotone-mover** |
| transportation.iata_code | 6 | 8 | 8 | 11 | +5 (+83.3%) | +3 (+37.5%) | regressed |
| contact.calling_code | 2 | 2 | 2 | 2 | 0 (0.0%) | 0 (0.0%) | flat |
| location.continent | 2 | 2 | 2 | 2 | 0 (0.0%) | 0 (0.0%) | flat |
| transportation.icao_code | 1 | 0 | 1 | 1 | 0 (0.0%) | 0 (0.0%) | flat |
| coordinate.latitude | 1 | 1 | 1 | 1 | 0 (0.0%) | 0 (0.0%) | flat |

## Band summary

| band | count |
|---|---:|
| monotone-mover | 4 |
| v22-jumper | 0 |
| flat | 9 |
| regressed | 1 |
| mixed | 0 |

## Reading this

- **monotone-mover** subtypes responded to the v20/v21/v22 boundary-training campaign as a whole; another retrain in the same shape would likely continue moving them.
- **v22-jumper** subtypes only responded to v22's specific recipe (whatever distinguishes v22 from v21 — typically the boundary-blend ratio or the embed-branch contribution). The v22 recipe was the load-bearing change for these subtypes.
- **flat** subtypes were not addressed by any of v20/v21/v22. Their bottleneck is something other than boundary blend — candidate explanations: thin training volume, header-pattern thinness, or a fundamentally different miss class (header-only signal, sibling-context dependence).
- **regressed** subtypes worsened. Flag these for next-spec diagnostic before any further boundary-training spend.
