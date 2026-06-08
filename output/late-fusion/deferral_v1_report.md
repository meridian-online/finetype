# B3 deferral selector v1 — offline efficacy probe

**Features:** `fusion_feats_v26`  (100046 rows, 64.2% base/expert disagree)

**Architecture:** v19 floor + HistGBM override selector on 968-dim cached features. Override fires iff base≠expert AND P(override-helps) ≥ τ.

## Headline

- **v19 floor (held-out):** 0.6253
- **fused (τ=0.95):** 0.6279  →  **Δ +0.0026**
- override coverage **0.33%** of all columns, precision **86.4%** (+48 net correct)

## Threshold sweep (held-out)

| τ | fire | coverage | override-precision | net | fused_acc | Δ vs floor |
|---|------|----------|--------------------|-----|-----------|------------|
| 0.30 | 3471 | 17.35% | 13.0% | -2569 | 0.4969 | -0.1284 |
| 0.35 | 3058 | 15.28% | 14.2% | -2190 | 0.5158 | -0.1095 |
| 0.40 | 2699 | 13.49% | 15.5% | -1863 | 0.5322 | -0.0931 |
| 0.45 | 2413 | 12.06% | 16.6% | -1611 | 0.5448 | -0.0805 |
| 0.50 | 2109 | 10.54% | 18.1% | -1345 | 0.5580 | -0.0672 |
| 0.55 | 1815 | 9.07% | 20.0% | -1089 | 0.5708 | -0.0544 |
| 0.60 | 1554 | 7.77% | 22.1% | -868 | 0.5819 | -0.0434 |
| 0.65 | 1283 | 6.41% | 24.1% | -665 | 0.5920 | -0.0332 |
| 0.70 | 1050 | 5.25% | 26.7% | -490 | 0.6008 | -0.0245 |
| 0.75 | 805 | 4.02% | 29.8% | -325 | 0.6090 | -0.0162 |
| 0.80 | 553 | 2.76% | 35.8% | -157 | 0.6174 | -0.0078 |
| 0.85 | 268 | 1.34% | 57.1% | +38 | 0.6272 | +0.0019 |
| 0.90 | 165 | 0.82% | 67.3% | +57 | 0.6281 | +0.0028 |
| 0.95 | 66 | 0.33% | 86.4% | +48 | 0.6277 | +0.0024 ◄ |

## Per-class recall Δ at chosen τ (val classes ≥10 cols)

Top gains (override recovers starved/collapsed types):

| label | base | fused | Δ | n |
|-------|------|-------|---|---|
| representation.text.plain_text | 0.615 | 0.642 | +0.027 | 1924 |
| geography.location.city | 0.766 | 0.771 | +0.005 | 1000 |

Losses (override hurt a class): **1**

| label | base | fused | Δ | n |
|-------|------|-------|---|---|
| representation.text.entity_name | 0.846 | 0.845 | -0.001 | 3663 |

## Read

**PROMISING — improve-or-hold holds offline.** The floor guarantees Δ≥0 in expectation at high precision; the real test is whether per-class gains land on the Sharpen-unreachable boundaries and survive the corpus-honest gate (rare-label relocation).


## Eval stem: `feats` (240 cols)

- v19 floor **0.467** → fused (τ=0.95) **0.467**  (**+0.000**); selector fired on **0** cols

Per-label base-vs-fused recall (the boundaries B3 targets):

| label | base | fused | Δ | n | fired | expert-right@fire |
|-------|------|-------|---|---|-------|-------------------|