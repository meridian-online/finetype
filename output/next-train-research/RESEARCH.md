# Next training run — research brief (Sense-gold 0.75 + lower latency)

**Date:** 2026-06-26 · 9-agent workflow (6 map → 2 adversarial verify → synthesis) · every number below
reproduced from source/preds this session (predict_multibranch re-run + score_gold_anchor re-score).
Raw workflow output archived in the session task file `wweze7fmb.output`.

---

## Verdict (one line)

0.75 *exact-type* accuracy from the learned model is structurally unreachable (best-ever 0.571,
perfect-2-encoder oracle ceiling **0.599**); the real win is a two-stage classifier head that lifts
raw Sense to ~**0.60–0.66** plus a **free** batch-path speed-up — and the product number analysts
already see (composed) is **0.812**, so first confirm we're measuring the right thing.

## Why 0.75 raw Sense is out of reach (three hard bounds, all reproduced)

1. **Best Sense ever measured = 0.571** — and that's one fragile seed (gte two-view s42; seeds 43/44
   were 0.536/0.524, robust 3-seed mean ~0.544).
2. **Perfect 2-encoder oracle = 0.599** (`twoview_result.md:13`) — picking the better of two strong
   contextual encoders per column still caps below 0.60. 0.75 is +0.15 above that wall.
3. **56% of error is the choice-0096 over-tighten pathology** — gold is a loose "no tighter type fits"
   label, the flat softmax picks a tighter attractor. 249/445 errors. Flat-softmax ceiling if that's
   unfixable = 1 − 249/931 = **0.7325**, and that's the *optimistic* ceiling (assumes every sibling +
   genuine miss fixed perfectly).

Shipped `m2v8m-s43` reproduced at **486/931 = 0.522** raw Sense, **736/931 = 0.791** composed.

## Canonical Sense ceiling table (verified)

| candidate | encoder / arch | Sense | composed | latency | what it isolated |
|---|---|---|---|---|---|
| v19 (retired) | potion-4M 128d, 240-label | 0.502 | 0.793 | ~0.39ms | tuned reference |
| m2v-244-s44 | potion-4M, 27-stat, 244 | 0.521 | 0.769 | ~0.39ms | reproducibility baseline |
| **m2v8m-s43 (SHIPPED)** | dual-encoder potion-8M | **0.522** | **0.791–0.812** | ~free | encoder capacity FLAT vs 4M |
| potion code-16M | code-aware teacher | 0.524 | 0.776 | ~free | teacher diversity FLAT |
| static two-view | 8M ++ code-16M, 2048d | 0.513 | 0.770 | ~free | static fusion doesn't transfer |
| per-value attention | 8M + cross-value self-attn | **0.561** | 0.793–0.817 | +2.3ms/col | cheap contextuality, composed FLAT |
| gte two-view | frozen ++ ft transformer | **0.571** | 0.787 | ~100× | best-ever; ties v19 composed |
| **2-encoder oracle** | max(frozen, ft) per-col | **0.599** | ~0.85 | n/a | **hard structural ceiling** |
| cdist (44-stat) | numeric-distribution stats | **0.316** | 0.685 | ~0.39ms | **the 44-stat collapse — DO NOT repeat naively** |

**Composed is rule-bound** (3 independent confirmations): 8M flat, gte ties v19, per-value attention
flat 0.793. A raw-Sense win on the numeric/over-tighten mass is largely redundant — Sharpen already
owns loose-vs-tight (latitude Sense 0.18 → composed 1.00).

## The error, decomposed (445 gold misses)

| bucket | cols | share | encoder-fixable? |
|---|---|---|---|
| over-tighten (0096) — loose gold → tighter attractor | 249 | 56% | **no** (flat softmax can't) |
| sibling/structural (coord→decimal etc.) | 146 | 33% | partly (attn fixed ~25/81) |
| genuine miss (unix→npi etc.) | 150 | 34% | only contextual encode |

Over-tighten splits into text-residual (124: word 68, plain_text 32, alnum 24) and loose-numeric
(139: integer→binary **52**, →numeric_code 32, →year 17; lat/lon→decimal 68).

## What 0.75 could mean (the fork)

- **Raw leaf-exact Sense** (literal): 0.522 now, ceiling ~0.60 → **unreachable**.
- **Composed / product accuracy**: **already 0.812** (s43) / 0.817 (attn + 0.6.37 rules) — banked.
- **Domain-level Sense**: **0.709 today** / composed 0.868 — a scoring change, no retrain.

## The reachable win

**[Sense] Two-stage / abstaining head on the existing static encoder** — coarse family/domain head
+ a calibrated commit-to-leaf-vs-stay-loose gate that removes the residual from flat-softmax
competition. Only lever touching the 56% over-tighten mass; latency-neutral. Projection **0.60–0.66**.
Load-bearing risk: does it **recover** the mass or merely **relocate** it (six prior additive retrains
all relocated)? Pilot single-seed before the 3-seed burn.

**[latency] Free batch-path plumbing (do regardless):** the enrichment taxonomy is reloaded and 245
validators recompiled *inside* the per-file loop (`profile.rs:243,257` under loop at `:150`) despite a
one-time compile at `:124`. Hoist it + read N files per duckdb spawn → ~halves the ~80ms/file batch
marginal, ~5–6h saved per 500k-file corpus pass. Predictions identical.

**[latency] Modest free win:** collapse dual encoder → single potion-4M (Sense flat 0.521 vs 0.522,
−20–40ms load). *Caveat:* the 0.769-vs-0.794 composed gap is confounded — needs a clean A/B in the
same retrain or it's a blocking composed regression.

## Recommended next run (ranked)

0. **[both] Confirm the target** (XS) — raw / composed / domain. Flips everything.
1. **[latency] Ship batch-path plumbing now**, decoupled from the retrain (low). Spec under card 0006.
2. **[Sense] Build the two-stage head** on static potion (high — *this is the run*). Pilot first.
3. **[Sense] Numeric value-range features into the GATE only, not the flat softmax** (med). **Do NOT
   bump COLUMN_STATS_DIM 27→44 naively — that's the cdist 0.316 collapse.** Pilot for no-collapse.
4. **[latency] Collapse to single potion-4M** in the same retrain + run the confound A/B (bundled).
5. **[Sense] Free rider:** A/B `--logit-adjust-tau 0.5–1.0` (trivial — implemented, never enabled).
- **Parallel data bet (separate spec):** corpus-mined header+values vocab-membership corpus (GeoNames,
  ISO-3166-2, Wikidata Q5, airports) lifted *gte-tiny contested* 0.648→0.82, never tried on a static
  multi-branch. Highest-value untested idea; must clear drift-proxy + corpus-honest (the gte fine-tune
  on it broadened the residual attractor: representative −5.8pp, corpus-honest NO-GO ×2).
- **Do NOT:** bigger static encoder (flat); transformer/gte backbone (0.599 oracle, 100×, ties v19);
  naive 44-stat; per-value attention unless target is confirmed raw-Sense and latency is secondary
  (it costs +2.3ms/col for a composed-flat gain).

## Open contradiction to flag

The training-pipeline agent calls numeric-range stats a +0.10–0.15, free, high-confidence win. The
ceiling + untried-levers agents weight the **cdist 44-stat collapse to 0.316** heavily. Resolution:
numeric range genuinely helps under-commit numerics (lat/lon→decimal) but those are Sharpen-owned
(composed flat), and the over-tighten integers (integer→binary) may *worsen*. Feed the gate, pilot,
don't trust the optimistic projection.

## What we don't know yet

- Does the two-stage head recover or relocate the over-tighten mass? (single load-bearing unknown)
- Does any over-tighten Sense gain pass through to composed, or is it Sharpen-redundant?
- Are the 52 integer→binary / 32 integer→numeric_code "errors" gold-debatable conventions?
- Will the header+values clean-data lever transfer to static multi-branch or broaden the attractor?
- Released-binary load floor (build.rs embed) vs disk-load path measured here.
