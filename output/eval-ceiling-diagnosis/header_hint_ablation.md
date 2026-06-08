# Do the hardcoded header hints make other outcomes harder? — corpus-scale ablation

**Date:** 2026-06-09
**Question:** are the deprecated hardcoded header hints net-helping or net-hurting at corpus scale, and does the defer-fix capture the upside without the harm?
**Method:** three full passes on the 33k stratified sample with ONE binary — hints **ON** (A), **OFF** (B, via RHH_DISABLE_HINTS=all-families), **DEFER** (C, FINETYPE_HINTS_DEFER=1) — scored on the rare-type scoreboard + the corpus-honest gate. The first time this has been measured on instruments that can see corpus-scale rare-type effects; the April RHH ablation used the curated 448-row eval and explicitly skipped sibling-context.

## Answer: mixed — net-harmful in bulk, load-bearing in spots, and **not removable wholesale today**

Both removing and deferring the hints are **NO-GO on the blocking corpus-honest gate.** So your instinct is half-right: the hints *do* hurt in aggregate, but you cannot simply switch them off — the model isn't yet strong enough to stand alone on the types they prop up.

### 1. In bulk, the hints push predictions *away* from the oracle

| config | oracle agreement (707k cols) | predictions changed vs A | `unknown` marginal |
|---|---:|---:|---:|
| A hints-ON | 0.4927 | — | 414,938 |
| B hints-OFF | **0.5294 (+3.7pts)** | 9.3% | 614,372 (+48%) |
| C hints-DEFER | 0.5114 (+1.9pts) | 4.8% | 701,742 (+69%) |

Turning hints off **raises** bulk oracle agreement by 3.7 points. The mechanism is visible in the `unknown` column: the hints were **forcing confident labels where the model would honestly abstain** — remove them and the model says "unknown" 48% more often, which both agrees with the oracle more *and* aligns with the Precision Principle (fewer confident mislabels). That is real, measurable bulk harm from the hints.

### 2. But they are load-bearing where the model is weak — removing them regresses

| rare type | A ON | B OFF | C DEFER |
|---|---:|---:|---:|
| **url recall ↑** | **0.925** | **0.340** | 0.683 |
| latitude FP-rate ↓ | 0.0012 | 0.0028 | 0.0026 |

And the gate caught the cross-type damage:

- **B hints-OFF → NO-GO.** `datetime.epoch.unix_seconds` **collapses** 39,799→17,637 (correct_ratio 0.475 — a *real* loss); `technology.internet.data_uri` **over-emits 8.8×** (2,073→18,211 — without the url hint the model floods data_uri); plus `isbn`, `postal_code`.
- **C hints-DEFER → NO-GO**, but milder: 2 triggers vs 4. Still `data_uri` over-emit 8.8× (defer doesn't help here — the model is *confidently* wrong on data_uri, and defer only stops overriding confident predictions) and an `isbn` oracle-FP.

So the hints are genuinely doing work the model can't yet do for **url, datetime epochs, and isbn** — exactly the "model-gap" families the April RHH roadmap flagged as removal-blocked.

### 3. The defer fix is strictly better than removal — but too blunt applied globally

DEFER beats OFF on every axis (4.8% vs 9.3% churn; url recall 0.683 vs 0.340; 2 gate triggers vs 4; +1.9 vs +3.7 oracle but without as much collateral). It's a sound idea. But applied to **all** hardcoded hints uniformly it still trips the gate, because it can't distinguish the bulk-harm families from the load-bearing ones.

## Recommendation: surgical, not wholesale

The data says: **defer/remove the families where the model is strong; keep the families where it isn't.** That's precisely the April RHH classification (12 removable / 10 model-gap), now confirmed and *quantifiable at corpus scale* with these instruments. Concrete path:

1. Apply the defer fix **per family** — defer the bulk-forcing hints (the `header_hint_table` finance/representation arms driving the +3.7 bulk gain), **keep** url, datetime-epoch, and isbn hints until the model covers them.
2. Re-run this exact ablation per-family to find the configuration that captures the bulk-agreement gain **and** clears the gate.
3. Long-term, the real fix is the RHH roadmap's other half: fortify training data so the model-gap families become model-covered, then retire their hints.

## What this does NOT change
Nothing shipped. `models/default` untouched; both flags default-off; this is measurement. The global defer fix stays a flag-gated tool (useful per-family), not a default.

## Evidence
`scripts/ablation_score.py`; gate reports `gate_hints-{B,C}.json` (both NO-GO); pass parquets `abl_{A,B,C}/` (gitignored, ~280MB each). Binary built with `--features finetype-model/rhh-instrumentation`.
