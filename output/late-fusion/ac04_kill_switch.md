# ac-04 — pre-port kill switch: **NO-GO**

**Date:** 2026-06-08
**Spec:** 2026-06-08-late-fusion-sense-classifier
**Verdict:** HALT. Do not port the head to Rust (ac-05–ac-08 are blocked by the
pre-committed halt condition).

## The headline

The fusion model, asked to classify the 240 hardest-confusion gold columns, gets the
**exact families the whole spec set out to fix backwards**. It sends every latitude
column to "decimal number", every categorical column to "country code". The model that
was meant to *gain* on the starved geography classes instead *loses* the ones the shipped
v19 already gets right.

## Evidence — gold anchor, per-family accuracy

| Family | v19 (shipped) | fusion-v25 | Δ |
|--------|--------------:|-----------:|----:|
| A_tight_code_vs_alnum    | 0.167 | 0.400 | **+0.233** |
| B_country_vs_categorical | 0.967 | 0.500 | **−0.467** |
| C_lat_lon_temperature    | 0.867 | 0.344 | **−0.523** |
| D_year_vs_integer        | 0.667 | 0.617 | −0.050 |

Per-label recall, the collapses:

| Curated label | v19 recall | fusion recall |
|---------------|-----------:|--------------:|
| geography.coordinate.latitude     | 1.000 | **0.000** |
| geography.coordinate.longitude    | 1.000 | **0.033** |
| representation.discrete.categorical | 0.967 | **0.000** |

Where the misroutes land (fusion prediction on the curated columns):
- latitude × 30 → `representation.numeric.decimal_number` × 30
- longitude × 30 → decimal_number × 29, longitude × 1
- categorical × 30 → `geography.location.country_code` × 30

This is a per-family regression on three of four families — itself a pre-committed HALT
condition. The corpus-honest gate was **not** run: the gold-anchor regression alone is
blocking, and a corpus-honest pass requires the single most expensive operation in the
plan (the full feature dump over the 33k-file stratified sample). Avoiding that spend on a
already-proven regression is exactly what this kill switch exists to do.

## Why it failed — the mechanism

The architecture is not the problem. The arch-test proved a value-level CharCNN *can*
learn the starved families: trained on manufactured coordinate diversity it reaches
latitude 0.729 / longitude 0.813 (ac-01, `eval_v25_prod.json`). View1 carries a real
lat/lon signal.

The **head's training corpus** is the problem. The residual head was trained on the
distilled sherlock corpus, which inherits sherlock's class distribution — it contains
**no coordinate columns at all**. Throughout ac-03 the lat/lon families showed `NaN`
recall in validation precisely because no such column existed to validate against. So:

1. The head never saw a gradient connecting View1's lat/lon activation to the lat/lon
   output. It learned the boundaries of the classes the distilled corpus *does* contain —
   decimal_number, country_code — and routes anything numeric-looking there.
2. The learned residual weight settled low (α = 0.139). That α looked healthy on the
   distilled val set ("the head trusts its MLP"), but it is exactly what kills the rare
   geography preservation: the v19 logits — which alone call latitude correctly — are
   scaled down to 14% and overwhelmed by the head's confident wrong routing.

In short: a general replacement trained on a corpus that omits the target families cannot
learn those families, and the residual is too weak to inherit them from v19.

## Recommendation — fusion is salvageable; the training substrate is not

Do **not** abandon the late-fusion architecture. The value-level view works. The fix is to
train the head on a corpus that **contains the rare/starved families it is meant to
repair** — the same manufactured-coordinate diversity that made the ac-01 value-CharCNN
succeed — rather than the distilled sherlock corpus, whose class distribution is the very
gap B3 exists to close.

Concretely, the next bet is: rebuild the fusion feature dump over a corpus that blends the
distilled columns **with** manufactured coordinate / categorical columns (so lat, lon and
categorical have non-NaN support in head training), retrain + re-search the head, and
re-run this same kill switch. Only a head that holds the gold families clears it.

Until then: keep multi-branch v19 as the shipped Sense default. No port, no promotion, no
version bump.
