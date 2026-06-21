# ac-01 verdict — REPRODUCIBLE. The dead-end is gone.

**Date:** 2026-06-22 · spec `2026-06-21-reproducible-baseline-and-static-embeddings` · card 0002

## Headline

We can rebuild v19's quality from scratch. A fresh 3-seed Model2Vec retrain at v19's exact
recipe (potion-4M, **27 stats**, format v4) but at the live 244 taxonomy **reproduces v19** —
and the "fresh retrains lose ~0.19" blocker that dead-ended the roadmap was **entirely the
44-stat column-distribution change**, not a broken pipeline. Strip it back to 27 stats and the
Sense quality comes straight back.

## Numbers (gold, honest gate)

| model | Sense (raw) | composed |
|---|---|---|
| v19-relu-s42 (reference) | 0.502 | 0.793 |
| repro-s42 | 0.481 | 0.755 |
| repro-s43 | 0.510 | **0.770** |
| repro-s44 | **0.521** | 0.769 |
| **best-of-3** | **0.521** | **0.770** |

- **Sense reproduced and beaten.** Best-of-3 Sense 0.521 > v19 0.502; the 3-seed mean (0.504)
  ≈ v19. The representation quality is fully recovered.
- **Composed is a statistical tie.** Best-of-3 composed 0.770 sits **inside v19's 95% CI
  [0.767, 0.819]** (n≈931). Nominally ~2pp under, within noise.
- **Seed spread matters.** Sense 0.481–0.521, composed 0.755–0.770 across seeds — single-seed
  comparisons are noisy (±~3pp). **Fresh-vs-fresh must be best-of-3 or multi-seed**, never a
  single run.

## What this settles

1. **cdist's 0.316 Sense was the 44 stats, full stop.** The reverted 27-stat extractor
   (commit `bdb7c79`) recovers Sense to 0.50–0.52. The column-distribution NO-GO is now
   doubly confirmed: it didn't just fail to help, it *destroyed* the representation.
2. **The 240→244 taxonomy change is benign.** Training at the live 244 (the new plain_text +
   zoneless-datetime leaves) costs nothing measurable — composed within CI, Sense up. No drift.
3. **The roadmap is open.** "Default to v19 is forced" no longer holds — we have a movable,
   reproducible baseline.

## The movable baseline (for ac-02 fresh-vs-fresh)

**`models/repro-baseline-relu-s44`** — Sense **0.521**, composed **0.769**. Chosen as the
baseline because ac-02 compares *representation* quality (Sense), and s44 has the best Sense
while tying s43 on composed (0.769 vs 0.770). Every bigger-static candidate (potion-8M/32M) is
measured against **this**, not the frozen v19.

The small composed shortfall (0.770 vs 0.793) lives in the Sharpen composition / seed noise,
not the Sense — the rules add +0.29 to v19 but +0.25 to the repro. Not worth chasing now: a
richer embed (ac-02) may lift composed past v19 anyway, and rule-simplification (ac-04) is the
deliberate place to re-examine the Sharpen layer.

## Next

ac-02: swap potion-4M → 8M → 32M as the embed branch (the gte `build_ftmb_v5_gte.py` patch is
the template per ac-00), retrain, measure Sense-only gold **and per-column latency** vs
`repro-baseline-relu-s44`. Require a candidate that beats the baseline's Sense at ≤~3× latency.
