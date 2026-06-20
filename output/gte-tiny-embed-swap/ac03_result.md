# ac-03 — offline standalone gold gate (gte embed swap)

**Headline: NO aggregate win. The fine-tuned encoder REDISTRIBUTES accuracy, net flat-to-slightly-negative.**

| model | encoder | standalone gold | 95% CI |
|---|---|---|---|
| gte-floor-s42 | frozen gte-tiny | 0.532 (495/931) | 0.500–0.564 |
| gte-release-s42 | fine-tuned gte-small | 0.518 (482/931) | 0.486–0.550 |

CIs overlap → statistically tied. NOT comparable to v19's 0.798 (composed; standalone omits the Sharpen
rule layer worth ~+0.27 — see memory ac03-standalone-vs-composed-gate). predict_multibranch validated
(reproduced 0.9497 on the floor's own training FTMB).

## The redistribution (per-family delta, release − floor)
Helped (semantic-text types): currency_amount +0.44, unix_seconds +0.40, url +0.27, tight_code/alnum +0.13, city +0.06, utc +0.11
Hurt (numeric-shape types): **latitude −0.47**, postal −0.40, year_vs_integer −0.15, region −0.14, isbn −0.14, longitude −0.09
28% of predictions changed (668/931 identical).

## Read
The 20× separation probe (frozen +0.058 → fine-tuned +1.195) measured IDEALISED semantic separation and did
NOT translate to aggregate gold accuracy. The fine-tuned semantic encoder helps where the embed is the binding
constraint (semantic value types) and HURTS numeric-shape discrimination (latitude/postal/year) — a
redistribution, not a lift. Confirms the pre-registered risk: the embed branch is not v19's binding constraint;
upgrading it (even dramatically) nets ~zero because the other branches dominate and numeric types regress.
The latitude/postal collapses are exactly the relocation the corpus gate (H05) exists to catch.

## Verdict
NO-GO on the wholesale embed swap. Fall back to the validated narrow escalation (+1.1/flat). The valuable
residue: a map of WHERE a semantic encoder helps (semantic-text types) vs hurts (numeric-shape) — input to any
future hybrid. Caveat: standalone only; composition (ac-04) could heal some shape-type losses via Sharpen, but
the flat aggregate does not justify building it.

## Ablation (predict_multibranch --zero-embed) — confirms the mechanism
Silencing the embed branch on the release model (char/stats/header/validation only):
- Aggregate: release 0.518 -> zeroed 0.412 (embed carries real net signal).
- BUT format/structural types RECOVER when the misleading embed is silenced:
  utc 0.54->1.00, numeric 0.43->1.00, year 0.92->1.00, postal 0.10->0.40, alnum-id 0.38->0.57.
- Semantic types crater: city 0.76->0.06, codes 0.97->0.17, currency 0.61->0.44.
- The ft-embed is RIGHT-where-zeroed-wrong on 190 cols, WRONG-where-zeroed-right on 92 cols.
  -> it overrides char/stats and corrupts ~92 format columns the structure knew.

## The prize (oracles, standalone)
| selection (per column) | standalone gold | composed est (+~0.27) |
|---|---|---|
| floor (frozen embed) | 0.532 | ~0.80 |
| release (ft embed) | 0.518 | ~0.79 |
| two-view: max(frozen, ft) | 0.599 | ~0.85 |
| gate: max(ft, structure-only) | 0.617 | ~0.87 |

A model that learns WHEN to trust the semantic embed vs defer to structure has a ~0.60-0.62
standalone ceiling -> ~0.85-0.89 composed, CLEARLY beating v19's 0.798. Wholesale swap is dead;
embed GATING / two-view is the live bet.
