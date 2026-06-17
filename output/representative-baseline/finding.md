# Representative-data accuracy baseline (v19, 0.6.33 full pipeline)

**Date:** 2026-06-17
**Question (from memo `2026-06-17-enum-accuracy-reframe-0.7.0`, precondition):** Gold's ~0.80
is a contested-curated *hard* slice. Where does the shipped model actually sit on
representative production data — and is 98% reachable on this architecture?

## Headline

**On representative data the model scores ~0.68 — *not higher* than the curated
gold slice (0.79), if anything lower.** Gold is not a uniquely-hard instrument that
understates production; the opposite is closer to true. 98% is not reachable on this
architecture. The model lives in the 0.65–0.80 band wherever you point it.

In plain terms: profile a random production table and roughly **one column in three
comes back with the wrong type** — and the misses are not exotic. They are usernames
called full names, 10-digit timestamps called medical-provider IDs, UUIDs called
postal addresses, and small vocabularies the model can't recognise as "just a
category".

## Method (verified end-to-end)

- **Sample:** 260 uniform-random **non-trivial** columns drawn deterministically from
  the 6.59M-column gittables corpus (`columns.parquet`; 35.6% of corpus columns are
  trivial/constant and excluded, matching gold's scope). 250 scored (10 dropped as
  harness key-lookup artifacts — timestamp/numeric column *names* that don't round-trip
  as schema property keys, + 1 CSV parse error; excluded from num and denom, no bias).
- **Model:** shipped default `sherlock-v19-relu-s42`, run through a freshly-built
  **0.6.33** binary (full Sense+Sharpen pipeline — the installed 0.6.25 lacks the four
  0.6.27–0.6.29 Sharpen fixes). Same `_profile_column` mechanism as the gold harness:
  feed the same ~8 sample values, read `x-finetype-label`.
- **Ground truth:** blind Sonnet panel labelled every column from values+header only
  (no model prediction shown), choosing from the 240-leaf taxonomy — the same evidence
  the model and gold's own llm-tier saw. Then **two independent blind Opus adjudicators**
  ruled on all 88 model/panel disagreements, shown both candidate labels without being
  told which was the model's.
- **Provenance check:** corpus-pass predictions were generated when `models/default`
  pointed at v19 (git: symlink set 2026-04-27, unchanged until 2026-05-25; corpus pass
  ran 2026-05-22). CLAUDE.md's "ran against v22" names the training *target*, not the
  predictor. Sidestepped entirely by re-running the shipped model fresh.

## Numbers

| Instrument | What it is | v19 accuracy |
|---|---|---|
| **Representative (this study)** | uniform-random corpus columns | **0.68** (adjudicated band 0.680–0.688; raw panel 0.648, CI 0.587–0.705; high-confidence-panel-only 0.683) |
| Gold corpus | curated, contested *hard* slice | 0.79 fresh / 0.80 of-record |
| 448 manifest | curated handpicked | 0.82 |

Adjudication outcome on the 88 disagreements: **78 the model lost on the merits**, 8 the
panel was wrong, 2 genuine ties. So the panel was a fair referee, not a harsh one.

A second, *independent* estimate landed in the same place: weighting gold's
per-predicted-type precision by the corpus prediction frequency gives ~0.66. That method
is formally **invalid** (gold's per-type composition is selection-biased toward each
type's failure cases, so it is not transportable to production) — but it pointed the same
direction, which is reassuring rather than load-bearing.

## Where the errors live (78 model-losses, by family)

| n | family | addressable? |
|--:|---|---|
| 24 | **categorical residual missed** — small vocab emitted as a specific type / word / entity_name | hard model limit ([[cardinality-boundary-error-is-real]]); residual can't be a flat-softmax class ([[categorical-is-a-residual-category]]) |
| 13 | other (word/plain_text/entity_name/datetime-variant boundaries) | mixed |
| 12 | **numeric_code vs integer** on ID columns (GAME_ID, PLAYER_ID, subject_id…) | genuinely contestable boundary |
| 12 | **full_name over-emitted on usernames** ("Author" columns of login handles) | **value-based rule** (handle-shape) — corroborated on gold (full_name P=0.167) |
| 7 | **gross value-shape collisions** — UUID→full_address, Message-ID→full_address, epoch→npi, "0.0"→url, EAN→integer, EDT→iata_code | **value-shape veto rules** — clean signatures |
| 4 | Sharpen veto → `unknown` destroyed a reasonable answer (Count, num_likes, CARD) | possible over-veto |
| 3 | unix epoch not recognised ("Created" 10-digit) | **value-range rule** (the originally-named target) |
| 3 | boolean vs numeric (0/1 columns) | contestable |

## What this means for the Sharpen objective (reframe)

The memo named three value-boundary targets: utc/unix-timestamp, year-vs-integer, url
over-emission. Representative data re-prioritises them:

- **year-vs-integer is a non-issue** — year is already healthy (P=0.929 / R=0.975 on gold);
  it did not appear in the representative loss set at all.
- **unix epoch is real but small** (3 cols) — and overlaps the gross-collision family
  (epoch→npi, both 10-digit).
- **url over-emission is small** here (1 gross case, "0.0"→url).

The **larger, cleaner, value-addressable masses are different**:
1. **username recovery** (full_name→username) — 12/250 representative, and directly fixes
   gold's worst precision leak (full_name P=0.167). Single highest-value value-based rule.
2. **structural shape vetoes** — UUID, Message-ID, and 10-digit-epoch shapes are being
   swallowed by full_address / npi. Each has a trivially clean value signature.

Both are 0048-shaped (value-based, not header-hint) and gold+corpus-honest gateable.

## Caveats (honest scope)

- The 0.68 rests on a fresh single-session panel — more label noise than gold's
  multi-round adjudication. The **robust** claim is the *band*: representative sits in
  0.65–0.80, not the ~0.95 that "gold is a hard slice" would predict. The exact point is
  softer than the conclusion.
- n=250 supports the headline and the big error families (categorical 24, username 12,
  ID-boundary 12); the small families (epoch 3, boolean 3) are directional only.
- Artifacts: `panel_labels.json` (labels+confidence), `repr_predictions.tsv`
  (column_name + prediction), this file. Raw values were not persisted (sanitised output).
