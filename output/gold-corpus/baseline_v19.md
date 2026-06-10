# Gold corpus v1 — shipped v19 baseline (ac-06)

**Date:** 2026-06-10 · **Fixture:** `eval/gold/gold_corpus_v1.tsv` (589 verified columns:
240 anchor + 349 lens-consensus + 0 adjudicated — grows as the author's sitting lands) ·
**Model:** `models/default` → sherlock-v19-relu-s42, real Sense→Sharpen path ·
**Full tables:** `report_v19-gold-corpus-v1_2026-06-10.md` · **Scorer:** `score_gold_anchor.py`
(extended: `build-gold` fixture merge, external vendored-CSV predict, Wilson CIs, global per-label metrics)

## Headline

**v19 gets 404 of 589 human-or-consensus-verified columns right — 68.6% (95% CI 64.7–72.2).**
This is the first accuracy number in the project scored against verified labels rather than
a proxy oracle. It is the number future candidates must beat.

## What drags it down (the stories, in user terms)

- **Postal code is the worst over-emitter measured so far: precision 0.074.** When v19 says
  "postal code" on this corpus, it is wrong 25 times for every 2 it is right — five-digit
  integers and zip-shaped quantities absorb the label. (FineType says postal; validation
  would then reject most rows — the round-trip failure mode.)
- **Identifier columns vanish: alphanumeric_id recall 0.128.** v19 demotes 34 of 39 real
  id columns (to unknown or specific codes) — the anchor's A-family finding, now confirmed
  at corpus diversity.
- **Plain integer recall is 0.390** — not because integers are hard, but because contested
  strata pull them into utc/year/postal/etc. This is the over-emit collateral measured
  column-by-column instead of via oracle proxies.
- **city/region/tld/text recalls are weak** (0.2–0.33 on small supports) — consistent with
  the known Sense blind spots.

## What holds up

Coordinates (P=1.0, R=1.0 — Sharpen's value-range rules earn their keep), dates
(iso P=1.0), decimals, year (P=0.947), country_code (P=0.971), unix_seconds, data_uri.

## Honest scope limits

1. **349 of 589 labels are lens-consensus, not yet human-checked.** The author's 350-column
   sitting (in progress) both adds adjudicated columns and spot-validates the consensus
   mechanism. Treat sub-stories on small supports as provisional until then.
2. **utc has support 1** — the corpus's only verified true utc-offset column is external
   (OpenFlights). The utc battle remains FP-side only; that scarcity is now a measured
   fact, not an assumption.
3. **Predictions run on the truncated sample values** (same path as the anchor baseline) —
   comparable across models, but not identical to a full-file profile.
4. **Selection bias is by design**: half the corpus was drawn where v19 emits contested
   types, so 68.6% is NOT "v19's accuracy on random tables" — it is accuracy on the
   contested + backbone mix the sizing memo specified. Compare models on this fixture,
   not this number to other corpora.

## Re-run

```
python3 scripts/score_gold_anchor.py build-gold
eval/gittables/.venv/bin/python scripts/score_gold_anchor.py predict \
  --gold eval/gold/gold_corpus_v1.tsv \
  --columns output/ydf-validation-gate/v19_gated.parquet \
  --binary target/release/finetype --out output/gold-corpus/predictions_<model>.tsv
python3 scripts/score_gold_anchor.py score --gold eval/gold/gold_corpus_v1.tsv \
  --predictions output/gold-corpus/predictions_<model>.tsv \
  --model-name <model> --out-dir output/gold-corpus
```
