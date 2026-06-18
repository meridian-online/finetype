# build ac-01 — region fix + working end-to-end fine-tune

**Spec:** 2026-06-18-minilm-encoder-build · **Date:** 2026-06-19
**Scope:** gte-tiny, family-level (8 classes), 244-col contested test. Directional — the
production 250-class model + corpus-honest gate still close ac-01/ac-02.

## Both gaps from the first cut are closed.

| model | contested acc | region recall | residual P/R |
|---|--:|--:|--:|
| shipped multi-branch | 0.684 | — | — |
| frozen-head (region sacrificed) | 0.836 | 0.00 | 0.91 / 0.78 |
| frozen-head (region-fixed) | 0.783 | 0.73 | 0.85 / 0.75 |
| **end-to-end fine-tune (v3)** | **0.811** | **0.73** | **0.88 / 0.78** |
| zero-shot ceiling | 0.893 | — | — |

## 1. Region — fixed (recall 0.00 → 0.73)

The admin1-only vocab missed gold's region values; the fix was **matching + data**, not more
generic vocab:
- **Format-robust matcher**: case-fold + strip admin suffixes ("County/Parish/Borough/…") from
  both vocab and values, add NYC boroughs, strip ISO-3166-2 prefixes (US-PA→PA). Gold region
  *value* coverage 24/90 → **65/90**; columns matched 12/15.
- **Targeted mining**: 9,000 real region/county columns mined from the corpus (header-confirmed),
  labelled via the improved matcher → ~3.5k clean region training examples (was ~600).
- Region is a multi-admin catch-all (counties, boroughs, districts) — the residual after the fix
  (abbreviations like "Mat North") is genuinely hard and partly a gold-label question.

## 2. End-to-end fine-tune — now works (and balances region vs residual)

The frozen head forced a trade: recover region (0.783, residual over-applied) OR maximise overall
by abandoning region (0.836, region 0.00). The fine-tune resolves it — **0.811 with region 0.73
AND residual held (0.88/0.78), every family working.** That's a better *real* model than the
0.836 that only looked good by giving up on region.

**Two failures, then the recipe that works:**
- v1 (full fine-tune, class-weighted, lr 2e-5): **NaN** on step 1 (MPS instability + heavy weights).
- v2 (+ grad-clip + warmup + lr 1e-5, full fine-tune, class-weighted): **catastrophic collapse** —
  trained but destroyed the pretrained features, predicting one class, loss rising.
- **v3 (works): freeze lower layers, train top-2 + head, discriminative LR (encoder 2e-6 / head
  1e-3), NATURAL cross-entropy (no class weights), grad-clip 1.0, warmup.** Loss 0.55→0.30→0.22,
  stable, all families recalled. 3.5M trainable params, ~65s/epoch on M1.

**Lessons (load-bearing for the production fine-tune):**
- Don't full-fine-tune a tiny encoder on a narrow task — it forgets. Freeze lower layers + tiny
  encoder LR.
- Heavy inverse-frequency class weighting destabilises; natural CE on the (roughly balanced)
  assembled data is stable. Calibrate residual-vs-specific via thresholds/the gate, not loss weights.
- ac-02 attractor: residual precision 0.88, no collapse — holds through the fine-tune.

## Where ac-01/ac-02 stand

Recipe fully validated: real mined columns + vocab-membership labels (incl. format-robust region) +
gte-tiny + gentle partial fine-tune → **0.811 contested, region recovered, no attractor.** What
remains to *close* the ACs: scale to the production 250-class escalation head, and run the
corpus-honest relocation gate (the arbiter; these 244-col numbers are directional). The region tail
(abbreviations) and pushing toward 0.893 (more epochs / more region+iata data) are incremental.
</content>
