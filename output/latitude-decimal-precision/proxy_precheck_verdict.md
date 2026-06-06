# ac-03 — destination-drift proxy pre-check: **GO**

Spec `2026-06-06-latitude-decimal-hard-negative-retrain`, ac-03 (gate). The
proxy's **first forward use** (2-for-2 retrospectively; never before run on a
blend we then intended to promote).

## Verdict

**GO — no label drifted beyond the calibrated band** (`|Δrate| ≥ 0.40pp AND
move ≥ 3.0× --direction up`). The overnight 3-seed run (ac-04) is authorised.

- Proxy: 1 seed × 10 epochs on `output/multibranch-training/latdec-blend.ftmb`.
- Wall-clock: **27 min** (best val_acc 88.8%) — well under the ~20% overnight target.
- Baseline: `sense_dist_v19fx_s42.json` (fixed 1,000-file list, n=13,533 cols).
- Drift report: `output/destination-drift-precheck/proxy_drift_latdec-proxy.json`.

## The load-bearing boundaries — the ones v23/v24 exploded

| label | base | cand | Δpp | × | read |
|---|---|---|---:|---:|---|
| geography.coordinate.latitude | 0.13% | 0.16% | +0.022 | 1.17× | **flat** — v24 blew this 4.3×; the inverse bet leaves it alone |
| geography.coordinate.longitude | 0.17% | 0.24% | +0.067 | 1.39× | flat |
| representation.discrete.categorical | 1.71% | 2.93% | +1.219 | 1.71× | under band — v23 blew this 4.69× |
| representation.numeric.decimal_number | 31.29% | 32.42% | +1.131 | 1.04× | **TARGET moved UP** — intended direction |

The target (decimal) rises; the two failure boundaries from the 0-for-2 record
(categorical, latitude) stay put. The withdrawal hypothesis — pulling feature
floats OUT of an over-grabbing class perturbs the softmax less than injecting
INTO one — holds at proxy depth.

## What this does NOT yet tell us

The proxy measures *destabilisation*, not *efficacy*. It confirms the blend is
safe to spend an overnight run on; it does NOT confirm the C-family fix lands.
Whether decimal recall actually climbs from 0.600 and latitude precision from
0.714 is ac-04's gold-anchor re-score, at full 3-seed / 50-epoch convergence.
The effective hard-negative dose (~1,388 of 2,540, from the pooled cap) is the
first knob if the gold anchor underwhelms.
