# ac-02 potion-8M (single-view) verdict — flat Sense, real but marginal composed gain

**Date:** 2026-06-22 · spec `2026-06-21` ac-02 · 3-seed, embed_dim 1024 (potion-8M, 4×256)

## Headline — capacity alone doesn't buy Sense; it buys a small composed bump

A bigger single-view potion (8M) does **not** improve the representation's Sense over the
m2v-244 (potion-4M) baseline — but it does compose ~3pp better, landing at v19's level.

| metric | m2v-244 (4M) | potion-8M | delta |
|---|---|---|---|
| Sense best-of-3 | 0.521 | 0.522 | **+0.001 (flat)** |
| composed best-of-3 (same compose path) | 0.765 | 0.794 | **+0.029** |
| latency | ~0.39 ms/col | ~free (encode flat across sizes) | ~0 |

Per-seed (Sense / composed): 8M s42 0.487/0.764, s43 0.522/0.792, s44 0.493/0.794. The
composed gain holds on **both** scored seeds (s43 +2.7pp, s44 +3.6pp vs m2v-244), so it is
not a single-seed fluke — though at n≈927 (CI ±2.7pp) it sits right at the edge of significance.

## Verdict vs the pre-registered rule

- **Sense gate: NO-GO.** The rule was "GO if best-of-3 Sense > 0.521 + CI." Sense is flat — the
  "richer embed -> better Sense" thesis does **not** land for single-view 8M.
- **Composed: improved (and ties v19).** 0.765 -> 0.794 ≈ v19's 0.793. Apples-to-apples
  (both via compose_predictions; compose_predictions understates native by ~0.4pp, so native
  potion-8M composed ≈ 0.80). A genuine, if marginal, composed gain.

## The interesting bit — flat Sense, better composed

8M's Sense is no more *accurate* overall, but it is more *rule-composable*: the Sharpen stack
adds +0.27 to 8M (0.522->0.794) vs +0.24 to m2v-244 (0.521->0.765). The wider embed shifts
*where* the Sense errors fall — toward types the value-based rules can recover — without moving
the headline Sense number. Consistent with the composed-loss analysis: the gap was always more
rule-shaped than embed-shaped.

## Why this was the expected shape

It matches the gte evidence: gte **single-view floor** Sense was 0.532 — barely above v19's
0.502 — and the real lift came from **two-view** (0.571). Capacity alone (a bigger single
potion) tests the weak axis. 8M flat-Sense confirms it. **The lever is two-view, not size.**

## Decision — go to two-view, skip 32M single-view

- **NOT a ship on its own:** composed only ties v19; Sense flat. Not worth the corpus-honest
  gate yet.
- **Skip potion-32M single-view:** it's just more capacity on the axis 8M just showed is flat —
  low expected Sense value.
- **Run two-view (4M ++ 32M) next** (`m2v-tv-4m32m`, embed_dim 2560, config staged): the direct
  test of the axis that carried gte's gain. If two-view lifts Sense *and* composed, that's the
  candidate; if it too is flat on Sense, the static frontier has topped out -> fastembed-rs ONNX.

One line: *a bigger single potion doesn't make the model smarter, just slightly tidier for the
rules — the real test is two views, not more weights.*
