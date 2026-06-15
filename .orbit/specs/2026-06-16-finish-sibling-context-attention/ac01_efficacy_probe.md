# ac-01 — sibling-context efficacy probe: GO (coordinates), with a scope correction

**Date:** 2026-06-15
**Script:** `scripts/probe_sibling_context.py` · raw: `ac01_probe_output.txt`
**Substrate:** all 931 gold columns resolved to their REAL source tables (809 from
`/Users/hugh/datasets/gittables/`, the rest from local `eval/datasets/`), siblings read
directly. No model, no network.

## Verdict

**GO — but the win is coordinates, not currency.** The probe's bar (ac-01: "at least
coordinates separate strongly") is cleared decisively. The probe also corrected the spec's
scope: currency is recoverable, but its lever is the column's OWN header, not sibling
context — so currency should not be carried by this spec.

## Boundary 1 — coordinates vs decimal: STRONG sibling signal

| group | resolved | has a coordinate-NAMED sibling |
|---|---:|---:|
| latitude | 39 | **95%** |
| longitude | 45 | **96%** |
| decimal_number (negative) | 97 | **2%** |

- **AUC(coord-named-sibling \| coords vs decimal) = 0.956** — near-perfect separation from
  one fact: *is there a sibling column named lat/lon?*
- **Value-only baseline AUC = 0.716** (the column's own in-[-90,90] range). Sibling
  context adds **+0.24 AUC** over what the column reveals alone.
- This is exactly the signal sibling-context attention reads: a latitude column almost
  always sits beside a column whose *header* says longitude. Attention over sibling header
  embeddings captures it directly.
- Note: the coord-*shaped* (value-range) sibling metric is weak (AUC 0.640) — 59% of
  decimal columns also have siblings that fall in [-180,180]. The discriminative signal is
  the sibling **header name**, not sibling value ranges. Design implication: the attention
  must lean on sibling header embeddings.

## Boundary 2 — currency vs numeric: the lever is the OWN header, not siblings

| signal | AUC (currency vs decimal/integer) |
|---|---:|
| target's own header is money-named (salary/price/…) | **0.964** |
| money-named **sibling** present | 0.655 |

- All 5 gold currency columns have a money-named header; only 7% of numeric columns do.
  The own-header separates almost perfectly.
- Siblings barely help (0.655) — plenty of numeric tables also carry money-ish headers.
- **So currency is header-decisive, not sibling-decisive.** v19 misses it not for lack of
  context but for training starvation (it never learned salary/price→currency, the family
  was starved). That belongs to the header+training path (`spec 2026-06-12-currency-
  variant-recognition`), NOT sibling-context. Loading currency onto this spec would have
  over-promised.

## Consequence for the spec

- **Coordinates are the proven sibling-context win** — and stand in for the broader class
  of *sibling-paired* types (anything whose disambiguator is a neighbouring column).
  Proceed to the FTMB-v3 build (ac-02) with coordinates as the lead, measured target.
- **Drop currency from this spec's promised wins.** It is real but header/training-shaped;
  keep it in the currency spec. Update ac-03's target list accordingly when building.
- Next sweep worth running before ac-02 locks scope: which OTHER gold boundaries are
  sibling-paired (e.g. city/region/country tables, start/end date pairs)? The probe
  harness extends to them cheaply.
