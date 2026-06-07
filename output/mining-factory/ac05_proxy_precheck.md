# ac-05 — destination-drift proxy pre-check: NO-GO (overnight train BLOCKED)

Spec `2026-06-07-reference-data-mining-factory`, ac-05. The mandatory pre-check
(`scripts/proxy_pretrain.sh`, CLAUDE.md) trained ONE seed for 10 epochs on the
manufactured-blend FTMB (`output/multibranch-training/mfg-blend.ftmb`, 180,947
records), snapshotted its Sense distribution on the fixed 1,000-file / 13,533-column
list, and ran the calibrated `drift_report.py` gate (`--abs-floor 0.0040
--rel-mult 3.0 --direction up`) against the v19 baseline `sense_dist_v19fx_s42.json`.

**VERDICT: NO-GO (exit 1). The full overnight train is BLOCKED.**

The proxy is NOT degenerate — best val accuracy 89.03% at epoch 9, on par with the
v24/v23 proxies and the full models. A healthy model that still does this on the
real corpus is destination drift, not under-convergence.

## What tripped (named-label discipline, per `proxy_retrocalibration.md`)

| label | base | candidate | Δpp | × | read |
|---|---:|---:|---:|---:|---|
| `representation.text.entity_name` | 3.38% (457) | **44.71%** (6,051) | +41.34 | 13.23× | LARGE-base explosion — real drift, not a small-base proxy artefact |
| `representation.alphanumeric.alphanumeric_id` | 0.04% (6) | 9.24% (1,251) | +9.20 | 192× | small base, but +9.2pp absolute is large |
| `geography.location.region` | 0.28% (38) | 5.48% (741) | +5.20 | 19.26× | a MANUFACTURED type over-emitting |
| `geography.transportation.icao_code` | 0.02% (3) | 0.54% (73) | +0.52 | 21× | small base — possible under-convergence noise |

The collapse alongside the explosion is the load-bearing context the band's
`--direction up` does not flag but the ranked report shows:

- `representation.numeric.decimal_number` 31.29% -> **0.29%** (-31.0pp)
- `representation.numeric.integer_number` 12.95% -> **1.16%** (-11.8pp)

## Diagnosis

The manufactured blend — text-shaped reference values (cities, regions, street
names, country names, plus coordinates rendered as strings) blended at the v19
recipe (`--ratio-distilled 0.5 --distilled-cap 600`) — pushes the model's prior
toward text/entity classification and **collapses numeric prediction**. The
untargeted boundary that explodes is `entity_name` (a label we did NOT
manufacture), with numerics swallowed. This is the same failure mode as v23
(categorical +529%) and v24 (latitude 4.3×): a fix that looks good on its own
target destabilises an UNTARGETED neighbour. safety_score is structurally blind
to it; the pre-check is not.

## Decision

Do NOT launch the overnight train. Manufacturing dissolved starvation at the
census/FTMB level (latitude 10 -> 959 training columns), but the blend RECIPE
over-weights text-shaped manufactured columns. Next move is a recipe redesign —
dial the manufactured contribution down so the base numeric distribution is not
swamped (lower per-type `--distilled-cap` for the manufactured types and/or lower
`--ratio-distilled`), then re-run the proxy BEFORE any overnight spend. The
manufactured corpus itself (ac-01–04) stands; only the blend ratio is at fault.

Evidence: `output/destination-drift-precheck/mfg-proxy.runlog`,
`proxy_drift_mfg-proxy.json`, `sense_dist_mfg-proxy.json`.
