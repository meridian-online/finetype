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

## Diagnosis — CORRECTED (first diagnosis was wrong)

**The first-pass diagnosis ("the blend over-weights text-shaped manufactured
columns — dial the volume down") was wrong, and the dial-down knob it proposed
could not have fixed this.** Two findings overturned it:

1. **The blend is per-type balanced, not volume-flooded.** `prepare_multibranch_data`'s
   v3 `blend_columns` targets `effective_spt × ratio_distilled` distilled + remainder
   synthetic PER TYPE (~600 + 600 = 1,200 each), and geography is capped by
   `DOMAIN_CAP_OVERRIDES = {"geography": 3000}`. No manufactured type swamps the base
   distribution by count. The `read_ftmb --stats` figures the first pass cited
   (region 5,084, city 4,436) are sibling/context occurrences, not training balance.

2. **The real cause is degenerate proximity grouping.** `materialise.blend()`
   *appended* the manufactured block verbatim, and materialise emits types in sorted
   order — so the 2,086 manufactured columns landed as 18 contiguous single-type runs
   (longitude 583, city 549, latitude 430, postal 339, …). The v3 FTMB builder's
   `group_distilled_by_proximity` (`prepare_multibranch_data.py:1938`) cuts table
   groups from 5–15 ADJACENT rows on the assumption that adjacent rows share a source
   table. A sorted manufactured block therefore becomes degenerate same-type
   pseudo-tables, and the sibling-context branch trains on columns whose neighbours
   are all identical labels — the opposite of a real mixed table.

That is what collapsed the proxy: the numerics did NOT relocate onto coordinates
(the v24 failure mode); they were swallowed wholesale into generic TEXT labels
(`entity_name` +41pp, `alphanumeric_id` +9.2pp, `plain_text`, `unknown`) because the
sibling branch saw structureless single-type blocks and the model fell back to its
broadest text prior. This is a **blend-construction bug**, not destination drift and
not a recipe-ratio problem.

## Decision — re-blend with interleaving, re-proxy

Do NOT launch the overnight train on this FTMB. Manufacturing dissolved starvation
at the census/FTMB level (latitude 10 -> 959 training columns) and that stands; the
fault was entirely in how the manufactured columns were spliced into the base stream.
`materialise.blend()` now **interleaves** the manufactured columns evenly through the
base distilled stream (shuffled to break the per-type runs, one column every ~49 base
rows), so each manufactured column lands inside a real mixed-type base proximity group.
Re-blend, rebuild the FTMB, and re-run the proxy BEFORE any overnight spend. The
dial-down (`--distilled-cap` / `--ratio-distilled`) is NOT the fix and was not run.

Evidence: `output/destination-drift-precheck/mfg-proxy.runlog`,
`proxy_drift_mfg-proxy.json`, `sense_dist_mfg-proxy.json`. Re-proxy evidence to follow
under `mfg-proxy2`.
