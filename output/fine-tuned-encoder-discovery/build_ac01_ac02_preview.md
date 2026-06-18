# build ac-01/ac-02 — bounded fine-tune PREVIEW (not the production build)

**Spec:** 2026-06-18-minilm-encoder-build (overnight autonomous preview of ac-01/ac-02)
**Date:** 2026-06-18 · MiniLM head on frozen all-MiniLM-L6-v2, M1 MPS
**Code:** `output/fine-tuned-encoder-discovery/finetune_preview.py`

## Scope (what this is and isn't)

A bounded probe of ONE question — training dynamics on the contested families. It is
**not** the production build: values-only (the distilled data is header-less), family-level
(not 250-class), a linear head on a frozen encoder (not an encoder fine-tune), no corpus
gate. Train = 15,885 distilled rows (real training substrate, capped 2,500/family); test =
the 244 gold/repr contested columns (clean split — distilled is sherlock-derived, gold is
gittables).

## Results

| setup | gold/repr acc | RESIDUAL behaviour |
|---|--:|---|
| shipped multi-branch (context) | 0.684 | over-emits specific types |
| zero-shot probe, header+values (ac-02) | 0.893 | — (CV ceiling) |
| zero-shot probe, values-only | 0.766 | — (CV ceiling) |
| **head trained on distilled, natural freq** | **0.648** | predicts 60/244 residual (truth 129); precision 0.97, recall 0.45 |
| **head trained on distilled, class-balanced** | 0.602 | predicts 51/244; precision 1.00, recall 0.40 |

Semantic recall held up well in both trained heads (country_code 0.98, city 0.84–0.88,
country 0.82–0.91, region 0.60–0.73) — the encoder finds those types.

## Three findings

1. **The residual attractor does NOT reproduce.** The multi-branch's 0-for-6 failure was
   *over*-emitting categorical/residual. Here the head *under*-predicts residual (60 vs 129)
   at 0.97 precision — the encoder+head does not collapse into the residual sink. Class
   balancing (the ac-03 precedence proxy) didn't help and slightly hurt. **So the attractor
   is not the binding risk for this architecture** — a real, encouraging update to ac-03's
   open question.

2. **Encoder separability re-confirmed** — values-only 0.766, header+values 0.893, both above
   the shipped 0.684. The signal is in the representation, as the discovery found.

3. **The binding risk is TRAINING-DATA COVERAGE, not the encoder or the attractor.** A head
   trained on the *current distilled data* reaches only 0.648 on gold — **below the shipped
   model and far below the 0.893 ceiling**. It still over-emits specific types on the hard
   residual columns (residual recall 0.45) because the distilled residual class doesn't cover
   gold's contested cases (`PA`-is-plain-text, `Si`-is-a-status-flag). The encoder *can*
   separate them (CV proves it); the training data doesn't *teach* the gold boundary.

## What this refines for the build (ac-01)

ac-01 is not "fine-tune MiniLM on distilled" — that under-delivers (0.648). To realise the
0.893 ceiling the build needs:

- **Headers in the representation** (worth +13pp here: 0.766 values-only → 0.893 header+values).
  The production model has a header branch; the fine-tune must use header+values, not the
  header-less distilled format as-is.
- **Training data that covers the contested residual** — `PA`/`Si`/`aranzebia`-style
  codes-and-abbreviations-that-are-plain-text. The current distilled + generator residual
  class doesn't represent them, so the head keeps over-emitting specific types. This is a
  data-mining / generation effort, and it is now the **top build risk** — above latency
  (solved on GPU) and above the attractor (didn't reproduce).

## Honest caveats

Values-only + linear-head + family-level understates what a header-aware, encoder-fine-tuned,
250-class model could do — the 0.648 is a floor for the naive approach, not a ceiling for the
build. But it cleanly relocates the risk: the encoder and attractor are not the problem; the
contested-residual training data is. The production build should lead with that.
</content>
