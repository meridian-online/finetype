# ac-06 — promotion decision: **NO-GO**

Spec `2026-06-06-latitude-decimal-hard-negative-retrain`, ac-06. Candidate:
`sherlock-latdec-relu-s42`. Instrument: the full gittables corpus pass against
the candidate (504,454 measure-half columns, 9.08h, `eval/gittables/
corpus_pass_latdec/`), read against the v19 baseline corpus pass.

## Decision

**Do not promote.** The lead bet looked like a clean win on every curated
instrument and is a **regression on the exact boundary it targeted** once measured
at corpus scale. The single genuine win it produced (a top-level-domain precision
fix) is incidental to the bet and does not offset the cost.

## The curated instruments all said GO — and all three are blind here

| instrument | scope | verdict | what it saw |
|---|---|---|---|
| gold anchor (ac-04) | 240 curated cols | GO | family C perfect, all 3 seeds |
| m-19 profile eval | 448 manifest cols | GO | **369 → 377 (+8, +1.79%)** |
| drift proxy + post-train (ac-03/05) | 1,000-file sample | GO | latitude flat (+6 cols) |

All three pass. m-19 even *improves*. The candidate is genuinely better on the
curated columns — the family-C fix is real where the fixtures can see it.

## The corpus pass says the fix did not generalise — it relocated

The bet's thesis (ac-01): v19 over-grabs latitude on feature-floats; pull them
out to decimal. The bet's own success metric is "columns with sense=latitude AND
ydf=decimal" — v19 had **3,974**; the candidate has **0**. On its face, total
success.

It is a mirage. Trace where the candidate's latitude predictions came from:

| | v19 | candidate |
|---|---:|---:|
| latitude predictions (corpus-wide) | 7,974 | **9,814** (+1,840) |
| of which sense=latitude AND ydf=decimal (the ac-01 FP metric) | 3,974 | **0** |
| v19=decimal → candidate=latitude (new calls) | — | **4,417** |

The candidate fixed the 3,974 YDF-visible false positives **and created 4,417 new
ones** — on columns where **YDF abstains (ydf=None)**, so they are invisible to
the very metric the bet was scored on. The FP metric hit zero not because the
errors were removed, but because they moved to where the oracle is silent.

What the 4,417 new latitude columns actually are — **0.9% have a geographic
name**; the rest are feature-floats, the same character v19 was mis-grabbing:

```
1,194  ver            (version numbers: 1.5, 1.1, 1.4)
  576  cam
  112  RealTime(ms)
  ~700 HitRate_*       (3F, 0.5F, 3R, t3P, ...)
   75  prev_term_gpa
   44  pKa SEM (calc)
   ... BSP, Hours, distribution_offset, genre_rhy
```

v19's feature-float latitude FPs were ~2,700 (3,974 pool minus ~1,280 true
`Lat/lat/latitude`). The candidate's are ~4,377 (4,417 × 99.1%). **The target
confusion is ~60% worse at corpus scale.** The retrain did not learn "feature
floats are decimal" — it memorised the ~1,388 surviving hard-negative patterns
and shifted the boundary, pushing a different, larger set of feature-floats
(`ver`, `cam`, `HitRate`) *into* latitude. This is the v24 failure mode
(decimal→latitude over-emit), recurred and merely displaced.

## The one real win — incidental, not the bet

`technology.internet.top_level_domain`: **87,542 → 3,038** (−84,504). The dropped
columns are not domains — they are columns like `Type` with values
`comment`/`story`; YDF labels **99.4%** of v19's TLD set as categorical. v19 was
massively over-emitting TLD; the candidate sheds those false positives to `word`.
A genuine precision win — but we did not target TLD, it is a side effect of this
seed's convergence, and it cannot buy back a regression on the boundary we *did*
target.

## Secondary cost

`geography.location.country_code`: 6,196 → 4,712 (−1,484), scattering into
state_code / region / iata_code (adjacent geography, the gold-anchor/ac-05
near-miss pattern at scale) plus ~490 to word/alphanumeric (signal lost). Modest,
as forecast in ac-04/ac-05 — but a cost, not a wash.

## B08 read

- m-19 ≥ −0.5%: **PASS** (+1.79%) — but curated; structurally blind to the corpus FPs.
- no per-type ≥5% regression: **FAIL** — latitude precision regresses materially
  at corpus scale (4,417 new feature-float FPs).
- gate ≥ 0% / vci3: not computed — the latitude regression is already decisive; no
  gate-score outcome flips a NO-GO on the target boundary to GO.

## Methodology finding — the load-bearing lesson

Three gates passed and the corpus pass failed, for two compounding reasons:

1. **Rare-label undersampling.** Latitude is 0.13% of columns. The drift proxy's
   fixed 1,000-file list catches ~18 latitude columns — far too few to resolve a
   +23% corpus shift. The proxy is calibrated for over-emit *explosions* on
   common boundaries (v23 categorical, v24 latitude at 4.3×); a +23% drift on a
   0.13% label sits under its floor. Adds to the ac-05 entry against the proxy's
   forward-use record (which already mis-directed TLD).
2. **An oracle-gameable success metric.** "sense=latitude AND ydf=decimal" only
   counts FPs YDF can see. Relocating FPs onto ydf=None columns drives the metric
   to zero while the error count rises. Any future FP metric must either be
   unconditioned on the oracle being non-null, or track the oracle-abstain bucket
   explicitly.

The curated gold anchor remains the right *efficacy* instrument; it is not a
*generalisation* instrument. Corpus breadth is the only thing that catches a
boundary fix that relocates rather than removes.

## What the next bet starts knowing

The withdrawal hypothesis is not falsified, but a hard-negative *set* alone
overfits: it teaches specific patterns, not the class boundary. A regenerated bet
should (a) draw hard negatives across the *full* feature-float space (ver/cam/
HitRate/gpa families were absent from the pool), (b) gate on a corpus-scale
latitude-FP count that includes ydf=None columns, and (c) oversample latitude in
the proxy's file list so the pre-check can see the target boundary at all.
