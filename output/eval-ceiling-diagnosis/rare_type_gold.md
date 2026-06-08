# Prototype: an oracle-free precision metric the corpus gate can't see

**Date:** 2026-06-08
**Script:** `scripts/build_rare_type_gold.py`
**Purpose:** prove that a trusted, oracle-free metric on a contested rare type **moves across rounds** where aggregate corpus precision (~0.49 vs gated-YDF) is flat — closing the blind spot in `output/eval-ceiling-diagnosis/finding.md`.

## How the gold set is built (latitude boundary)

The contested rare types are *header-identifiable but value-ambiguous* — a latitude column and an RMS-error column have the same values; only the header separates them. So the **header is the trusted label**, validated against the values:

- **positives** — header is an unambiguous latitude name (`lat`, `latitude`, `y_lat`, `lat_dd`, …; anchored full-match, so `translate`/`platitude`/`latrine` are excluded) AND ≥90% of sample values are numeric with ≥80% decimals in [-90, 90]. → a genuine latitude column.
- **hard negatives** — header names a non-coordinate quantity (`population`, `score`, `error`, `rms`, `magnitude`, `temperature`, `elevation`, …) AND ≥90% of values are numeric. The header is a trusted *not-latitude* label; a model flipping it to latitude is a false positive.

No gated-YDF, no oracle — labels come from header + value validation only.

## Result — the metric moves where corpus precision doesn't

Scoring each round's `sense_prediction` on the gold set:

| model | lat+ cols | recall | hard-negs | **FP-rate** | precision |
|---|---:|---:|---:|---:|---:|
| v19 (shipped) | 2,461 | 0.996 | 192,596 | **0.0013** | 0.910 |
| v22 | 2,465 | 0.994 | 192,281 | **0.0072** | 0.647 |
| v23 | 2,450 | 0.995 | 191,447 | **0.0160** | 0.439 |
| latdec | 2,454 | 0.995 | 191,942 | **0.0023** | 0.882 |
| v0624 | 619 | 0.990 | 15,641 | **0.0010** | 0.971 |
| fusion_v27 | 618 | 0.994 | 15,218 | **0.0026** | 0.939 |

**Read FP-rate as the headline** — it's the fraction of non-coordinate numeric columns a round wrongly calls latitude, and it's directly comparable across passes (precision depends on the positive:negative ratio, which differs between the full-corpus passes v19/v22/v23 and the 33k-sample passes v0624/fusion_v27).

- **v22 and v23 badly degraded latitude precision** — FP-rate 0.0013 → 0.0072 → **0.0160 (12×)**; v23's latitude precision collapsed to 0.44. Recall never moved (~0.99), so the damage is pure over-emission, invisible to recall-based bars.
- **The recent precision patches recovered it** — v0624 (0.0010) and fusion_v27 (0.0026) are back at or below the v19 floor.
- Meanwhile **aggregate corpus precision read ~0.49 ±0.001 for every one of these rounds** (finding.md). The headline metric registered none of this swing.

## What this buys the next round

A round that touches the numeric↔coordinate boundary can now be measured on the *thing it changes* before promotion — FP-rate on the latitude gold set — instead of waiting for a 9-hour corpus pass whose headline number cannot move. It is also a fair scoreboard: v22/v23 would have been caught here pre-promotion.

## Extending it

`MODELS` is a list of `(label, parquet)`; add any pass with `column_name, sense_prediction, sample_values_truncated`. The latitude `POS_RE`/`NEG_RE` + value-range filter are the template for the other contested header-identifiable types — `url` (header `url`/`link` + URL-shaped values vs text/int negatives), `utc` (offset headers vs plain integers). A tighter variant restricts hard-negatives to value-confusable rows (decimals in [-90,90]) for a sharper, smaller-denominator FP-rate.

**Caveat:** header-anchored labels are high-precision, not perfect; this is a prototype scoreboard, not a hand-verified gold standard. The next step toward a canonical metric is a small human spot-check of the positives and the FP columns to confirm the header labels hold.
