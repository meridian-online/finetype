# Per-value attention — findings (ac-6, choice 0106, spec 2026-06-24-per-value-attention-pooling)

**Date:** 2026-06-25. **Run:** 1-seed pilot, `models/m2v8m-attn-s42` (potion-8M value
encoder, cross-value self-attention + PMA pool, n_values 50, 4 heads, 1 layer, 4 slots).

## Headline

FineType now reads a column nearly as well as a real transformer would, essentially
for free — and the shipped accuracy didn't move, because the bottleneck was never the
reading.

| | Sense | composed |
|---|---|---|
| v19 (retired) | 0.502 | 0.793 |
| m2v8m-s43 (shipped baseline) | 0.522 | 0.794 |
| **m2v8m-attn (seed 42)** | **0.561** | **0.794** |
| _gte transformer (reference, ~100× latency)_ | _0.571_ | _0.787_ |

## What attention bought

- **Sense +3.9pp (0.522 → 0.561).** ~80% of what a genuine contextual transformer
  (gte, 0.571) buys, at **~2.3 ms/column** (release CPU, batch=1; `value_attention::
  bench_latency_cpu_batch1`) versus gte's ~100×. Choice 0106's thesis — cheap
  cross-value contextuality via attention over static potion vectors — is **validated
  on Sense.** The architecture works.
- **Composed flat (0.794 → 0.794).** The Sense lift does **not** pass through to the
  metric that ships.

## Did it pass through to composed? No — and that's now load-bearing

Third independent confirmation that **composed is rule-bound**: bigger potion (flat),
gte (Sense +7pp, composed tied), and now per-value attention (Sense +3.9pp, composed
tied) all land composed at ~0.79. The Sense→Sharpen rule layer caps the shipped number
regardless of representation quality. There is now ~+4–7pp of Sense the rules
systematically fail to convert.

## Latency

Pool forward ~2.3 ms/column (release CPU) — roughly one extra Sense-classifier, not
gte's 100×. Adds no new encode (values already encoded for the blender). Bounded and
likely acceptable; native `finetype profile` confirmation deferred (moot without a
promotion). Note: training was ~8× slower/epoch on Metal (small-op batch inefficiency,
~21 min/epoch) — an *iteration* cost, not an inference cost; halvable by dropping the
double pool forward per batch (loss + train-accuracy) if a future run needs it.

## GO/NO-GO

- **NO-SWAP.** Composed flat is the documented no-swap outcome (ac-5), not a failure.
  No promotion → the corpus-honest relocation gate is not triggered.
- **GO on the rule-layer bet.** The real ceiling-break is Sense→composed, now
  evidence-backed: we have a representation (+4–7pp Sense across attention/gte) the
  rules can't cash in. That is a *separate, new spec*, not this one.

## Caveats / scope

- Single seed (0.561). The pre-registered gate wants best-of-3; a 3-seed run is ~40 h
  at the current epoch cost. Worth confirming **only if** we pursue the rule-layer bet
  and need the attention model as its input.
- The inference path (`MultiBranchClassifier::embed_input_tensor`) is code-complete and
  contract-tested (save/load round-trip), but the score above came from
  `predict_multibranch` (training-side forward). Native-vs-predict_multibranch numerical
  parity (ac-3) was not run at scale — moot without promotion.

## Substrate

`output/embed-frontier/ac02_m2v8m-attn_result.md`, `models/m2v8m-attn-s42/`,
`output/embed-frontier/preds/m2v8m-attn-s42_{sense,composed}.tsv`. Builds on choice 0106,
the embed-frontier verdict (`output/embed-frontier/ac02_FINAL_verdict.md`), choice 0096
(rule layer is the composed cap).
