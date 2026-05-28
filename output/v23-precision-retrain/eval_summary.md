# v23 precision retrain — eval summary

Per spec `2026-05-27-v23-precision-retrain` ac-04 close evidence.

## Band verdict: **Failed**

| Component | Reading | Threshold | Verdict |
|---|---|---|---|
| FP-rate (top-6 cluster columns) | **−70.8%** | Met ≥ 50% | **Met** |
| Cell-2 vs v19 (gated baseline) | **+5.1%** | Met ≤ −10.4% (v22's level) | **Failed** |
| **Combined ac-04 band** | | either-fails rule | **Failed** |

## Read

The precision targets shrank as designed (three of six clusters by
92–96%), but v22's geography lift collapsed: country regresses
+70.3%, region +29.0%, city +14.1% vs v22 on the gated cell-2
metric. v23 is worse than the v19 baseline.

## Mechanism

The discrete.categorical signal trained from ac-01's 50k
categorical-target hard negatives didn't stay scoped to F/C/G-style
columns. v23 fires categorical on **548,409 columns** (vs v22's
87,105 — +529.6%), with ~48k of those drawn directly from columns
v22 classified as `geography.location.city`. The boundary the model
learned was "categorical is the right answer for many
low-cardinality string columns" — coarser than the spec assumed.

## Artefacts

- `per_cluster_fp_rate.md` — per-cluster trajectory + FP-rate band
- `cell_deltas_v23_vs_v22.md` — gated cell-2 trajectory + per-subtype
- `relitigation_memo.md` — full post-mortem, what we learned, what
  to NOT try next, candidate next bets

## Action

ac-05 Path C. v22 (`sherlock-v22-boundary-relu-s44`) remains the
default. No new spec opened automatically; the next bet's shape is
a design question.
