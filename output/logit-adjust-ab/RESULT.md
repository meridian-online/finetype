# MOVE 3a — logit-adjust A/B (predict-time, no retrain)

Model: `models/m2v8m-s43` (shipped default). Gold: `eval/gold/gold_corpus.tsv` (931 cols).
FTMB: `output/embed-frontier/gold_m2v8m.ftmb`. Predict: `target/release/predict_multibranch --logit-adjust <tau> --priors <p>`.
Compose (real Sharpen): `scripts/compose_predictions.py`. Score: `scripts/score_gold_anchor.py score` (legacy, no reframe).

## Headline

| tau | prior | raw Sense | composed |
|-----|-------|-----------|----------|
| 0.0 | —        | 498/931 = **0.535** | 810/931 = **0.870** |
| 0.5 | train    | 498/931 = 0.535 (0 pred changes) | — |
| 1.0 | train    | 498/931 = 0.535 (0 pred changes) | — |
| 0.5 | emission | 425/931 = 0.456 (326 changed) | — |
| 0.75| emission | 411/931 = 0.441 (488 changed) | — |
| 1.0 | emission | 387/931 = 0.416 (588 changed) | 767/931 = **0.824** |

- **train prior is uniform by construction** (classes balanced at 1200 → `tau·log(prior)` is a constant vector → argmax unchanged → 0 prediction changes at any tau). It cannot move anything.
- **emission prior** (decimal 217k, integer 98k, unknown 81k) does move argmax, but the raw Sense headline falls **monotonically** with tau. No tau improves it.

## Fixed vs broken (emission prior, vs tau00)

| tau | wrong→fixed | correct→BROKEN | net | of fixes: lat/lon |
|-----|-------------|----------------|-----|-------------------|
| 0.5 | 22 | 95  | −73  | 19 |
| 0.75| 42 | 129 | −87  | 32 |
| 1.0 | 47 | 158 | −111 | 33 |

Breaks 3–4× more columns than it fixes at every setting. ~70% of the few genuine fixes are the single lat/lon↔decimal boundary.

## Per-type (the named over-tighten buckets)

Raw Sense, tau00 → tau10_emis:
- decimal: P 0.483→0.763, R 0.768→**0.305** (precision-for-recall trade on the attractor)
- integer: P 0.960→1.000, R 0.375→**0.078** (integer wasn't over-emitting; just loses TPs)
- latitude: R 0.179→**0.949** (tp 7→37) but P stuck ~0.31 (fp 16→78 — floods latitude)
- word: R 0.220→0.066; alnum: R 0.613→0.226; entity_name: R 0.750→0.417 — all worse

**Composed (tau00 vs tau10_emis) — the shipped-product view:**
- latitude: **identical** (tp=39, P=0.975, R=1.000)
- longitude: **identical** (P=1.000, R=0.978)
- decimal: R 0.989→0.958 (worse) · integer: R 0.891→**0.823** (−13 TP) · alnum: R 0.774→**0.694** (worse)

The Sharpen layer already lifts raw 0.535 → composed **0.870** (+0.335) and already recovers lat/lon to perfect/near-perfect recall — deterministically, for free. Logit-adjust's one real raw win is redundant; its collateral survives Sharpen and pulls composed to 0.824.

## Verdict — AGAINST the move-3b two-stage-head pilot (as a prior/logit reweighting)

The cheap no-retrain proxy for "rebalance the head" net-degrades at every tau, raw and composed. Its only reliable win (lat/lon↔decimal) is already owned by the deterministic value-based Sharpen layer. A global class-prior scalar cannot separate "decimal that should be latitude" from "decimal that is genuinely decimal" — it moves both, so it destroys ~3 TPs per fix.

Any Sense-head intervention must beat a Sharpen layer that already delivers +0.335 over raw Sense and closes the addressable gap to composed 0.870 → the real headroom is 121 columns, not the 56% raw over-tighten mass. A two-stage head only earns its keep if it conditions on column **values** (which Sharpen already does) rather than class frequency. Recommend NOT pursuing 3b as a reweighting/temperature head.
