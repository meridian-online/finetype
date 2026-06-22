# ac-02 FINAL — the static embed frontier has topped out; composed is rule-bound

**Date:** 2026-06-23 · spec `2026-06-21` ac-02 · all candidates 3-seed, Sense-vs-Sense + composed vs m2v-244

## Headline — no embedding beats the baseline on what ships

We tried capacity, a domain (code) teacher, and two-view fusion. None lifts Sense above the
m2v-244 baseline, and none beats v19 on composed. **The Sense ceiling for static embeddings on
our task is ~0.52, and the composed ceiling is set by the rules, not the embedding.**

| candidate | embed_dim | Sense (best-of-3) | composed (compose_predictions) |
|---|---|---|---|
| m2v-244 (potion-4M, baseline) | 512 | 0.521 | 0.765 |
| potion-8M (single, capacity) | 1024 | 0.522 — flat | **0.794** (+3pp, ties v19) |
| base-8M ++ code-16M (two-view, teacher diversity) | 2048 | 0.513 — flat/below | 0.770 — flat |
| _(reference) v19 shipped_ | 512 | 0.502 | 0.793 |
| _(reference) gte two-view (transformer, ~100x latency)_ | 3072 | 0.571 | 0.787 (ties v19) |

## What each axis ruled out

- **Capacity (8M):** flat Sense. More weights on one view does nothing for accuracy.
- **Teacher diversity (code-16M):** flat Sense, and it *diluted* 8M's composed bump. A code-aware
  teacher did not separate our format/code-shaped losses better than bge-based potion.
- **Two-view fusion (base ++ code):** flat. The fusion that carried gte (frozen ++ fine-tuned,
  0.571) does not transfer to static potions — confirming the earlier caveat that potion's
  available diversity (teacher/size) is a *weaker* axis than gte's frozen-vs-tuned.

## The deeper finding — composed is rule-bound, not embed-bound

Even gte's genuine transformer Sense gain (0.571, +0.07 over v19) only **tied** v19 on composed
(0.787 vs 0.793). And single-view 8M's +3pp composed was rule-composability, not better Sense.
This matches the ac-02 prep exactly: of m2v-244's composed losses vs v19, only ~1pp was
embed-addressable; the bulk (alphanumeric_id->geohash ×14, etc.) is the 0096 residual-attractor
pathology — **a value-rule problem no embedding can fix.** So no embedding path, static *or*
transformer, beats v19 on the metric that ships.

## Decision

1. **Stop the embed-frontier chase.** Static is capped (~0.52 Sense); transformer buys Sense at
   100x but still doesn't beat composed. There is no shippable embed candidate.
2. **Do NOT pursue fastembed-rs ONNX.** It would buy gte's Sense at ~10x instead of 100x — but
   composed is rule-bound, so even gte's Sense ties v19. A cheaper route to a Sense gain that
   doesn't move the ship metric isn't worth building.
3. **The standing win is ac-01:** m2v-244 is the reproducible, movable baseline (Sense 0.521 /
   composed 0.769 ≈ v19). The roadmap is unblocked even though its embed extension didn't pay.
4. **Redirect to the rules — that's where composed actually moves.** The alphanumeric_id->geohash
   veto (14 gold cols, the single biggest composed lever; task t-00007be9...) and the other
   residual-attractor losses are the real path to beating v19 on the product metric.
5. **8M single-view +3pp composed (ties v19):** a curiosity, not a ship — it only ties, and would
   cost a 256-dim model swap + the Rust EMBED_DIM const bump for no net headline gain.

One line: *we proved the accuracy gap isn't in the embedding — it's in the rules; a richer
representation, static or transformer, can't beat what ships, so the next work is value-rules,
not encoders.*
