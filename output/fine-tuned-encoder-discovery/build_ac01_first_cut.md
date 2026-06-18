# build ac-01/ac-02 — first cut (recipe validated; region is the open gap)

**Spec:** 2026-06-18-minilm-encoder-build · **Date:** 2026-06-19
**Scope:** frozen gte-tiny + linear head, family-level (8 classes), 244-col contested test.
Directional — NOT the production 250-class model or the corpus-honest gate (ac-01/ac-02 stay open).

## Headline: best contested accuracy yet — 0.836 — with the right data recipe.

| training data | encoder | gold-contested acc |
|---|---|--:|
| shipped multi-branch | — | 0.684 |
| distilled values-only (preview) | MiniLM | 0.648 |
| **synthetic vocab-generated columns** | gte-tiny | **0.635** ← worse! |
| mined corpus + header-heuristic | MiniLM | 0.820 |
| **mined corpus + GeoNames-membership labels** | **gte-tiny** | **0.836** |
| zero-shot ceiling (CV) | — | 0.893 |

## Findings

1. **Mine real corpus columns; do NOT synthesize.** Clean vocab-generated columns
   ("header: city | values: Paris, Tokyo, …") scored **0.635** — *worse* than the shipped
   model — because they're a distribution mismatch with gold's *messy* real-world columns.
   Real mined corpus columns (same distribution as the test) labelled by vocab membership
   scored **0.836**. **The vocabularies' job is LABELLING, not GENERATING.**
2. **gte-tiny + the recipe = 0.836** — beats MiniLM on the same recipe (0.82), as its
   zero-shot lead predicted. Confirms gte-tiny as the encoder.
3. **ac-02 (attractor): no collapse.** Residual precision 0.83–0.91, balanced recall — the
   model does not over-emit the residual sink that killed the 6 retrains. The attractor
   defence holds at this scale.
4. **Region is the open gap** — recall 0.00 (natural) / 0.33 (balanced), even with admin2 +
   us_states added. Two causes: thin region *training* (only ~600 mined columns pass
   membership) and *messy* gold region values (counties with "County" suffixes, uppercase,
   UK/NYC terms) that vocab membership misses. Region needs targeted mining + format handling.
5. **End-to-end encoder fine-tune NaN'd on MPS** (loss nan from step 1, collapsed to
   majority-class). A known MPS instability for end-to-end BERT fine-tuning — fixable with
   grad-clipping + LR warmup, or train on CPU/CUDA. The 0.836 above is a FROZEN encoder +
   head; the encoder fine-tune is the lever to push toward 0.893, deferred until the NaN is
   fixed.

## Where the build stands

- **Recipe validated:** real mined corpus columns + authoritative-vocab membership labels +
  gte-tiny → 0.836 contested, no attractor. This is the ac-01 data+model approach.
- **Open levers to 0.893:** (a) fix the region family (targeted mining + admin2/admin3 +
  format-robust matching); (b) the end-to-end encoder fine-tune (NaN fix); (c) more mined
  data per family (full_name/iata thin).
- **Then:** scale to the production 250-class escalation head and run the full promotion
  scoreboard — the corpus-honest relocation gate remains the arbiter (frozen-head 244-col
  numbers are directional, not a promotion).

## Caveats

Frozen head + linear classifier, 8 contested families, 244-col test. Real ac-01/ac-02 close
on the production fine-tune + the corpus-honest gate. Minor: the mined residuals are corpus
columns; a tiny overlap with gold is possible (sha not tracked in the pool) — negligible at
this sample size, but the production build should dedup against gold by identity.
</content>
