# ac-04 — latitude→decimal candidate vs v19 gold anchor (3 seeds)

Spec `2026-06-06-latitude-decimal-hard-negative-retrain`, ac-04. Candidate:
`sherlock-latdec-relu-s{42,43,44}` (3 seeds × 50 epochs, ReLU+BN, the v19
architecture). Oracle: the gold anchor on the shipped default v19
(`metrics_v19-models-default_2026-06-05.tsv`). 240 curated columns; gold label
is ground truth, YDF not consulted.

## The headline — the paired win landed, perfectly, on every seed

Family **C_lat_lon_temperature** is now a clean sweep on all three seeds:

| label | v19 P / R | candidate P / R (s42=s43=s44) | move |
|---|---|---|---|
| geography.coordinate.latitude | **0.714** / 1.000 | **1.000** / 1.000 | FP 12 → **0** — precision fixed |
| representation.numeric.decimal_number | 1.000 / **0.600** | 1.000 / **1.000** | FN 12 → **0** — recall fixed |
| geography.coordinate.longitude | 1.000 / 1.000 | 1.000 / 1.000 | held |

The 12 temperature/feature-float columns v19 grabbed as latitude now read
decimal. One fix, both numbers, to perfect — identical across all 3 seeds, so
this is structural, not seed-luck. **Both ac-04 success conditions met:** decimal
recall up (0.600 → 1.000) AND latitude precision up (0.714 → 1.000), latitude
recall held 1.000.

## The one cost — a reproducible 2-column country_code drift

| family / label | v19 R | candidate R (all 3 seeds) | move |
|---|---|---|---|
| B_country / geography.location.country_code | 0.967 | **0.900** | FN 1 → 3 (−2 cols) |

The two newly-missed columns, identical on every seed:

- `country_id` → predicted `geography.location.region`
- `country`    → predicted `geography.location.country` (full name, not the ISO code)

Both drift to **adjacent geography labels**, NOT to numeric — the decimal
injection did not pull them; the softmax rebalanced *within* geography. These are
granularity near-misses (code vs name; country vs region), not category errors.
ac-04 names country_code as a must-hold, so strictly this is a regression — small
(2/30 on the curated-hard fixture), reproducible, and confined to geography.

## Everything else held

| family / label | v19 R | candidate R | note |
|---|---|---|---|
| B_country / categorical | 0.967 | 0.967 | held |
| D_year / year | 1.000 (P 0.938) | 1.000 (P 0.938) | held |
| D_year / integer_number | 0.333 | 0.333 | held (the integer silence is a separate bet) |
| A_tight / alphanumeric_id | 0.167 | 0.167 / 0.200 / 0.300 | small **bonus**, seed-variable |

Macro recall: v19 0.754 → candidate **0.796 / 0.800 / 0.812** (+~5pp) — the +40pp
decimal gain dominates the −6.7pp country dip.

## Verdict

The lead bet **worked**: the inverse (withdrawal) hard-negative retrain fixed
family C completely and reproducibly, with no numeric destabilisation — the v24
failure mode did not recur. The single cost is a 2-column, within-geography
country_code drift on the curated fixture.

**Open question for GO/NO-GO (ac-06):** is that country drift a curated-fixture
artifact (2 hard edge columns) or a corpus-scale cost? The proxy showed corpus
geography flat, but the gold anchor is the finer instrument. ac-05 — the
full-label-space post-train Sense-distribution drift vs v19, watching
country_code / country / region — is the instrument that decides it.
