# Gold corpus v1 — shipped v19 baseline (ac-06)

**Date:** 2026-06-10 (updated same day after the adjudication tier landed) ·
**Fixture:** `eval/gold/gold_corpus_v1.tsv` (915 verified columns: 240 anchor +
349 lens-consensus + 326 two-panel llm-adjudicated, author-accepted via 40/40
spot-check — 40 of those carry the author tier) ·
**Model:** `models/default` → sherlock-v19-relu-s42, real Sense→Sharpen path ·
**Full tables:** `report_v19-gold-corpus-v1_2026-06-10.md` · **Scorer:** `score_gold_anchor.py`
(extended: `build-gold` fixture merge, external vendored-CSV predict, Wilson CIs, global per-label metrics)

## Headline

**v19 gets 606 of 915 verified columns right — 66.2% (95% CI 63.1–69.2).**
This is the first accuracy number in the project scored against verified labels rather than
a proxy oracle, and the number future candidates must beat. (The first cut, before the
adjudicated tier landed, read 68.6% on 589 columns — the adjudicated rows are harder by
construction, being the queue's most contested cases.)

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
- **Categorical columns are half-invisible: recall 0.390 at support 100.** Real
  status/type/category vocabularies routinely land elsewhere — the single biggest
  miss-pool in the corpus.
- **State codes are a wholly unhandled family: precision and recall both 0.000** (support
  7) — v19 reads `CA`/`GA`/`TX` as countries or categoricals, never as
  geography.location.state_code.
- **city over-emits (precision 0.667) while catching every real city** — boroughs,
  counties and team abbreviations absorb the label; region precision 0.350 mirrors it.
- **tld/text recalls stay weak** (0.33 / 0.22) — consistent with the known Sense blind
  spots.

## What holds up

Coordinates (latitude P=0.975/R=1.0, longitude P=1.0/R=0.978 — Sharpen's value-range
rules earn their keep), iso dates (P=1.0), decimals (P=1.0/R=0.865), year
(P=0.930/R=1.0), country_code (P=0.959/R=0.825), boolean terms (P=1.0).

## Honest scope limits

1. **Provenance tiers:** 240 anchor + 40 author-spot-checked rows are human-grade;
   349 lens-consensus and 286 llm-adjudicated rows are machine-verified with measured
   trust (two-panel agreement, author calibration 40/40, Wilson 95% lower bound on
   panel-author agreement ≥ 0.91). 24 queue rows remain open (panel splits/unsures)
   plus a 572-row unreviewed backlog.
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
