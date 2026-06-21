# ac-01 — numeric-range representation: REFUTED by its own probe. Redirect to the attractor.

**The embed was never the bottleneck. The latitude regression is the `decimal_number`
residual-attractor (decision 0096), which is rule-shaped — not a representation gap.**

## The probe (real gold columns, 5-fold CV logistic regression, standardized)
223 columns across latitude/longitude/year/postcode/decimal:

| features | 5-class acc | coord-vs-decimal acc |
|---|---|---|
| numeric_range_features (32) | 0.829 | 0.841 |
| frozen gte-tiny embed (1536) | 0.810 | **0.852** |

Range features are NOT better than gte — gte is marginally *better* on the decisive
coord-vs-decimal task. A linear probe on the gte embed already separates coordinate from
decimal at 0.85 on gold. **The signal is in the features; the representation is not the gap.**
So building a range-feature view (ac-01's premise) cannot fix the regression.

## Where the gap actually is — the attractor (from the existing predictions)
The floor model predicts `decimal_number` 171× when only 94 columns are decimal — a 1.8×
OVER-prediction, precision 0.50. What gets swallowed is exactly the coordinates:
longitude 38, latitude 34. Two-view reduces it slightly (150×, latitude 28) but it persists.

The features separate coord-vs-decimal, yet the flat softmax defaults to the frequent
`decimal_number` class anyway. That is the residual-precedence pathology of decision 0096:
a "no tighter type fits" numeric class becomes a universal attractor the flat softmax cannot
resist, regardless of feature quality. No embedding (gte / two-view / range) fixes a softmax
attractor — 0096 establishes it is rule-shaped, not trainable into the softmax.

## Redirect (recommend revising the spec)
Stop treating this as a representation problem. The lever is the `decimal_number` attractor:
1. **Logit adjustment** — the trainer already exposes `logit_adjust_tau` (CLI flag, applied
   only in training, ZERO inference cost; default 0.0 = off). It down-weights frequent
   classes during training, directly countering the decimal_number frequency attractor.
   Cheapest first test: retrain with a non-zero tau, re-score the coord-vs-decimal confusion.
2. **Value-based decimal->coordinate promotion gate** (per 0096: value-based, gate-shipped,
   both-sides evidence + kill switch). ac-00 noted promotion is risky (coord-promote-guard
   prevents false coords), so this needs the column's value-DISTRIBUTION as evidence
   (bounded ~[-90,90], ~half negative, characteristic spread), not a single range check.

ac-01 (range features) and by extension the spec's representation framing (ac-02 two-view
gating, ac-03 static distillation) are chasing a symptom. The numeric_range_features +
probe are kept as substrate (scripts/numeric_range_features.py, probe_range_features.py) —
they may still help marginally as one input — but the spec should re-centre on the attractor.
