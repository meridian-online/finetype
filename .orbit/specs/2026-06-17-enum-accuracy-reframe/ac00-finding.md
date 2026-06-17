# ac-00 — settle the enum reframe by measurement

**Date:** 2026-06-17 · spec `2026-06-17-enum-accuracy-reframe` · gate AC

## (a) Residual policy — SETTLED

Re-adjudicated 73 of the 95 gold `representation.discrete.categorical` columns (the 73
with values in the corpus pass; blind 3-agent Sonnet panel, "if categorical were not an
option, what is this?"):

- **71 of 73 (97%) have NO real semantic type.** They are genuinely bounded vocabularies:
  stock-exchange codes (GER/PNK), sports-team abbreviations (POR/OKC/BOS), content-type
  tags (comment/story), status flags (passed/Static/Dynamic), dataset-specific category
  labels (Turkish food categories, threat-intel classes, dividend types).
- **Only 2 resolved to a real type** (a timezone-abbreviation column → `datetime.offset.iana`;
  a file-format discriminator → `representation.file.extension`).

This is direct confirmation of choice 0102: **categorical is a property, not a type.** The
honest residual is **`representation.text.word`** for short single-token vocabularies
(the large majority) and **`representation.text.plain_text`** for phrase-shaped vocabularies
(threat-intel categories, bug-status phrases).

**POLICY:** former-categorical → `text.word` (short tokens) / `text.plain_text` (phrases),
carried alongside the already-shipped `x-finetype-enum` bounded-domain flag. The handful that
adjudicate to a real semantic type take it.

## (b) Eval-reframe headline delta — MEASURED, neutral-to-positive (NOT −3)

Scored the current v19 gold predictions under two explicit scoring models (927 columns):

| scoring model | headline | vs legacy |
|---|---|---|
| LEGACY — categorical is a competing label | 0.789 | — |
| REFRAME-A — categorical→word on truth+prediction (minimal) | 0.793 | **+0.4** |
| REFRAME-B — residual-family collapse {categorical, word, plain_text, entity_name} (lenient) | 0.808 | **+1.9** |

On the 95 categorical-truth columns: legacy 47 correct → reframe-A 51 → reframe-B 59. Removing
the contest stops penalising the model for picking word/plain/entity on a residual column.

**The memo's feared ~−3 is not borne out.** The reframe is headline-neutral to mildly positive.
Its value remains definitional (ontology correctness + the production payoff below), but it is
**not an accuracy sacrifice** — which strengthens the case to ship it.

## Why it matters beyond the headline

`categorical residual` was the single biggest **production** error mass — 24 of 78 misses (31%)
in the representative baseline (`output/representative-baseline/finding.md`). The reframe
dissolves that contest definitionally rather than trying to win an unwinnable boundary.

## Limits (over-read discipline)

- REFRAME-A maps a `categorical` *prediction* to `word`; the true post-retirement prediction
  is the model's pre-Sharpen Sense label, which is sometimes a different residual or a specific
  type. So +0.4/+1.9 **bracket** the real delta; the exact number needs the ac-01 scorer applied
  after the ac-03 rule retirement. The robust claim is the **sign and magnitude band: ~0 to +2,
  not negative.**
- 22 of 95 categorical gold columns are external/author-tier without corpus values; the residual
  policy is inferred to extend to them (they are the same kind of bounded vocab) but not
  re-adjudicated here.
- No enum ground truth exists; the enum-dimension precision/recall axis (ac-01) is a separation
  measure, not an accuracy number.

## Verdict

Gate **cleared**. Residual = `text.word`/`text.plain_text` + enum flag. The reframe is
headline-safe (≈0 to +2). Proceed to ac-01 (reframed scorer) → ac-02 (gold migration) → ac-03
(retire categorical-emitting Sharpen rules, corpus-honest gated).
