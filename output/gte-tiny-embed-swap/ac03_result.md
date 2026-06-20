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
