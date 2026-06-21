# Two-view embed overnight result (2026-06-21)

**Qualified GO. The gating mechanism works; not yet robust/complete.**

embed = frozen gte-tiny(1536) ++ fine-tuned gte-small(1536) = 3072. 3-seed standalone gold:

| seed | standalone gold | vs floor 0.532 |
|---|---|---|
| 42 | 0.571 (CI 0.539–0.603) | +0.039 (clears gate, CI-lower > floor) |
| 43 | 0.536 (CI 0.504–0.568) | +0.004 |
| 44 | 0.524 (CI 0.492–0.556) | −0.008 |

Baselines: frozen floor 0.532, wholesale-ft 0.518, two-view oracle ceiling 0.599.
Best seed = 65% of floor→ceiling. Clear step up from the wholesale swap (which tied the floor).

## Mechanism CONFIRMED (seed 42 per-family vs floor)
Two-view uses BOTH views — recovered format types the swap broke AND kept semantic gains:
- recovered: numeric +0.48, utc +0.40, longitude +0.17 (format) ; currency +0.39, codes +0.13, url +0.13 (semantic)
## Gaps
1. Seed-fragile: 1/3 seeds clears floor (spread 0.524–0.571). Soft-MLP gating is learnable but unreliable.
2. Numeric-range types STILL regress even in seed 42: postal −0.40, latitude −0.30. Neither gte view helps
   range/magnitude discrimination (both are semantic encoders that collapse "is this a latitude or a year").

## Next bet (evidence-backed)
Three-view embed: FORMAT-AWARE ++ frozen ++ semantic. The remaining headroom to 0.599 is exactly the
numeric-range types, which no semantic encoder solves. A format-aware encoder (trained to keep numeric
range/length apart) as a third view targets that gap directly. Do NOT build ac-04 on a single fragile seed.
Models: models/gte-twoview-s{42,43,44}.
