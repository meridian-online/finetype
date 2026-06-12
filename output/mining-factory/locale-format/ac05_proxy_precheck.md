# ac-05 — locale-format proxy pre-check: NO-GO (overnight train BLOCKED) — but the curse broke

Spec `2026-06-07-reference-data-mining-factory`, ac-05, locale-format re-run
(currency amount-format variants + datetime weekday/long-month variants added,
commits c3a6dd1 / 570dcdd). The mandatory pre-check (`scripts/proxy_pretrain.sh`,
CLAUDE.md) trained ONE seed for 10 epochs on the locale-format blend FTMB
(`output/multibranch-training/mfg-blend-localefmt.ftmb`, 130,755 records), snapshotted
its Sense distribution on the fixed 1,000-file / 13,533-column list, and ran the
calibrated `drift_report.py` gate (`--abs-floor 0.0040 --rel-mult 3.0 --direction up`)
against the v19 baseline `sense_dist_v19fx_s42.json`.

**VERDICT: NO-GO (2 labels tripped). The full overnight train is BLOCKED.**

Proxy is healthy — best val accuracy **89.01%** at epoch 10, on par with the v23/v24/mfg
proxies. A converged model that still does this is destination drift, not
under-convergence. Wall-clock 29 min.

## The headline: the numeric-collapse curse is GONE

The prior three pre-checks on the original (geography-heavy) manufacture were
catastrophic and identical — `decimal_number` collapsed 31.29% → **0.29%** at full
dose AND at 1/6 dose, which is what closed ac-05 with "additive blend is dead,
0-for-3". **That did not happen here.** With the locale-format corpus, the numeric
prior held flat:

| label | v19 base | prior mfg light-dose (proxy3) | **locale-format (this run)** |
|---|---:|---:|---:|
| `representation.numeric.decimal_number` | 31.29% | 0.29% (collapsed) | **31.46%** (+0.17pp — HELD) |
| `representation.numeric.integer_number` | 12.95% | 3.36% (collapsed) | **12.63%** (−0.32pp — HELD) |
| `representation.text.entity_name` | 3.38% | 28.32% (exploded) | **5.08%** (+1.70pp — contained) |
| `representation.text.plain_text` | 6.26% | 23.35% (exploded) | **4.99%** (−1.27pp — HELD) |

The text-explosion / numeric-collapse failure mode that made the additive route look
structurally dead is absent. Spreading the manufactured mass across 34 types (the
13 new locale-format types are text/formatted-string shaped, not bare decimals) so
geography is a smaller fraction of the blend appears to be what lifted the numeric
prior off the floor. **The route is not dead the way ac-05 concluded.**

## What tripped — two small, localised boundaries

| label | base | candidate | Δpp | × | read |
|---|---:|---:|---:|---:|---|
| `container.object.json_array` | 0/0.00% | 335/**2.48%** | +2.48 | 671× | NEW emission off a ZERO base — model learned a json_array boundary from the formatted-string family and mis-fires it on ~335 real corpus columns |
| `identity.commerce.isbn` | 13/0.10% | 98/**0.72%** | +0.63 | 7.3× | small-base over-emit |

Neither is the wholesale prior-collapse of the prior runs. `json_array` is the
load-bearing trip (2.48pp); `isbn` is marginal. The manufactured currency values
carry no `[...]` literal (accounting uses parentheses, others use symbols/grouping),
so json_array is not a value-shape leak from the manufactured data — it is a learned
decision boundary the model now over-applies to real columns. Which corpus columns
need a per-column prediction dump to confirm (follow-up).

## Decision

Do NOT launch the overnight train (gate is blocking, H05). But this is a
**diagnose-and-retry**, not a dead end like ac-05's 0-for-3 close. The expensive
failure mode is solved; the blocker is now two specific, much smaller boundaries that
look tractable. Next move: dump the proxy's per-column predictions on the fixed file
list, identify the ~335 json_array + ~98 isbn corpus columns, and decide whether a
targeted fix (a json_array/isbn hard-negative in the blend, or a format-disambiguation
adjustment) clears them without re-collapsing the numerics. Re-proxy before any
overnight spend.

Evidence: `output/destination-drift-precheck/mfg-localefmt-proxy.runlog`,
`proxy_drift_mfg-localefmt-proxy.json`, `sense_dist_mfg-localefmt-proxy.json`.
Proxy model: `models/sherlock-mfg-localefmt-proxy-s42`.
