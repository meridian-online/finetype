# Gated baseline soak — observation window

Per spec `2026-05-26-v22-gated-direction-review` ac-05.
`ac_type: observation` — close-time-deferred.

## What this AC is watching

The gated YDF baseline (per spec `2026-05-26-ydf-validation-gate`)
is calibrated at the 50% pass-rate threshold with length-only
validations excluded. Both choices were conservative defaults.

A stricter threshold (e.g. 75%) or a re-enabled length-only path
would shift the gated v22/v19 cell-2 ratio. v22's headline number
(−10.4% vs v19) sits in the middle of the Partial band; calibration
sensitivity ±2pp would not flip it. Calibration sensitivity
> ±3pp would push it out — either back to Failed (< 10%) or
forward to Met (≥ 20%).

## Close condition

This AC closes when at least one of the following holds:

1. Two further corpus passes have run (for any reason — m-19 work,
   follow-up spec, future Sense candidate) and the v22/v19 gated
   cell-2 ratio sits inside [−8%, −13%]. Record the readings; close
   with "band held, decision robust".
2. A reading lands outside [−8%, −13%]. Record the reading; open
   a follow-up spec re-litigating the Option A pick if the drift
   warrants it.

The 2026-05-26 baseline reading is **−10.4%** (per
`output/ydf-validation-gate/cell_deltas_gated.md`).

## Readings (append as they land)

| date | corpus-pass source | v22/v19 gated Δ | note |
|---|---|---:|---|
| 2026-05-26 | output/ydf-validation-gate/cell_deltas_gated.md | −10.4% | baseline reading |
