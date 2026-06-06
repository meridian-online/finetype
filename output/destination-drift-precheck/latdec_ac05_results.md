# ac-05 — post-train Sense-distribution drift vs v19, full 50-epoch s42

Spec `2026-06-06-latitude-decimal-hard-negative-retrain`, ac-05 (the mandatory
post-train check). Candidate: `sherlock-latdec-relu-s42` at full convergence
(50 epochs). Baseline: v19 default on the fixed 1,000-file list
(`sense_dist_v19fx_s42.json`, n=13,533 columns). Same instrument as ac-03's
proxy pre-check — now run on the *real* model the proxy was forecasting.

## Verdict — the ship gate held: **GO**

`drift_report.py --abs-floor 0.0040 --rel-mult 3.0 --direction up` (the calibrated
over-emit gate that supersedes the watch block): **GO — no label drifted beyond
the band.** No untargeted boundary exploded UP at full convergence. The proxy
said GO at 10 epochs; the full run confirms GO. **The proxy's first forward use
is 1-for-1 on the gate it is calibrated for** — the v23/v24 over-emit failure
mode (categorical 4.69×, latitude 4.3×) did not recur, exactly as the proxy
predicted.

## Two reshuffles the up-gate doesn't adjudicate — both worth recording

The up-gate only catches over-emits, because a *drop* is a reshuffle and the
question is always "where did it go". Two labels dropped. Neither lands in a gold
family, so the up-gate is silent — but a promotion decision needs their valence.

### 1. country_code halved — corroborates the gold anchor, sub-threshold at scale

| label | v19 base | latdec s42 | move |
|---|---|---|---|
| geography.location.country_code | 18 / 0.13% | 8 / 0.06% | −0.07pp, 0.44× |
| geography.location.region | 38 / 0.28% | 45 / 0.33% | +7 cols |
| geography.location.country | 27 / 0.20% | 28 / 0.21% | +1 col |

The gold anchor (ac-04) flagged country_code 0.967 → 0.900, the two curated
columns drifting `country_id → region` and `country → country`. **The corpus
confirms the direction**: country_code loses ~10 columns, region/country gain
them — the same within-geography scatter, **no leak to numeric**. But at corpus
scale it is −0.07pp, well under the 0.40pp band. Real, reproducible, gold↔corpus
consistent — and small.

### 2. NEW — top_level_domain collapsed, and the proxy pointed the wrong way

| label | v19 base | proxy 10ep | full 50ep |
|---|---|---|---|
| technology.internet.top_level_domain | 163 / 1.20% | **257 / 1.90% (UP)** | **5 / 0.04% (collapsed)** |
| representation.text.word | 359 / 2.65% | 207 / 1.53% | 548 / 4.05% |

`--direction both` flags this NO-GO: TLD 163 → 5 (−1.16pp, 0.03×), the ~158 lost
columns reshuffling into `representation.text.word` (+189). The up-gate correctly
ignores it — word is the receiving label and it didn't explode UP past band on
*its* rate, and TLD itself only dropped.

**The calibration finding is the sharp part:** the proxy at 10 epochs had TLD
*rising* (257, +0.70pp); the full 50-epoch run *collapsed* it (5, −1.16pp). The
proxy mis-directed this label entirely — a 10-epoch snapshot caught it mid-
trajectory, before the head settled. This is the first recorded entry **against**
the proxy's forward-use track record, and it is precise: the proxy is a reliable
forecaster of the **up-gate** (over-emit explosions), the thing it is calibrated
and used for — it is **not** a reliable forecaster of an individual untargeted
label's full-convergence *direction*, least of all for drops.

## The target itself, at corpus scale

| label | v19 base | latdec s42 | read |
|---|---|---|---|
| representation.numeric.decimal_number | 4234 / 31.29% | 4231 / 31.26% | **flat** |
| geography.coordinate.latitude | 18 / 0.13% | 24 / 0.18% | +6 cols, tiny |
| geography.coordinate.longitude | 23 / 0.17% | 28 / 0.21% | +5 cols, tiny |

Decimal is already 31% of the corpus; the 12-column gold-anchor fix is a rounding
error here. The gold fixture is the fine instrument for the curated-hard columns —
corpus decimal was never broadly broken, so corpus-flat is the expected, healthy
reading, not a contradiction of the +40pp gold gain.

## What this hands ac-06

The ship gate (over-emit) is GO and the lead bet landed perfectly on the gold
anchor. But promoting a **new default** turns on two untargeted reshuffles the
up-gate is structurally blind to, and **both land in labels the gold fixture
doesn't cover** — so neither has a labelled valence yet:

- **country_code halving** — almost certainly benign (within-geography, sub-
  threshold, gold-corroborated as granularity near-misses).
- **TLD collapse (163 → 5)** — the load-bearing unknown. If v19 was over-emitting
  TLD (false positives on word-like tokens — `com`, `org`, `co.uk` *are* words),
  the collapse is a precision **win**. If those 158 columns were genuinely TLD,
  it is a recall **loss**. The reshuffle target (word) is consistent with either.

**Recommendation for ac-06:** do not promote on gold-anchor + ship-gate alone.
Run the canonical promotion instrument — the full corpus pass (m-19 / gate / vci3,
gated-YDF ground truth, B08) — watching `top_level_domain` and `country_code`
specifically. The gold fixture cannot adjudicate either reshuffle; the corpus pass
can. The TLD collapse is the one finding that could flip a promote GO to NO-GO.
