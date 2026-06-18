# Contested-residual data lever — cheap proof (it CONVERTS)

**Spec:** 2026-06-18-minilm-encoder-build (the top-risk prerequisite for ac-01)
**Date:** 2026-06-18 · MiniLM head on frozen encoder, M1 MPS
**Code:** `output/fine-tuned-encoder-discovery/{mining_proof.py, mining_pool.tsv}`

## Verdict: invest — the data + header lever converts, even with noisy heuristic labels.

| training setup | gold contested acc (244 cols) |
|---|--:|
| distilled values-only (the overnight baseline) | 0.648 |
| shipped multi-branch | 0.684 |
| corpus-mined header+values, natural | 0.811 |
| **corpus-mined header+values, balanced** | **0.820** |
| zero-shot ceiling (header+values, CV) | 0.893 |

A header-mismatch heuristic relabelled 11,207 of the mined specific-type predictions as
RESIDUAL (over-emission proxy). Trained on that + real header+values, a frozen-MiniLM head
reaches **0.82 on the gold contested set** — up from 0.648 (distilled values-only) and the
shipped 0.684, closing ~75% of the gap to the 0.893 ceiling.

Per-family (balanced): city 1.00, country 0.91, region 0.73, country_code 0.76; **RESIDUAL
precision 0.89 / recall 0.82** — balanced, no attractor collapse.

## What it proves

1. **Headers are load-bearing** — the lift from 0.648 (values-only) to 0.82 (header+values)
   realizes the +13pp+ header lever the discovery flagged. The build's representation must be
   header+values, not the header-less distilled format.
2. **Mining contested-residual examples teaches the boundary** — even a crude header-keyword
   heuristic for "this specific prediction has no header support → residual" is enough to lift
   the contested set materially. The corpus has the raw material (~570k contested-type
   predictions; balanced 2,500/family mined here).
3. **No attractor** — balanced training holds residual precision (0.89) without collapsing.

## Honest caveats (why this is a proof, not the build)

- **Noisy labels.** The header-keyword heuristic is crude; some relabels are wrong. Clean
  LLM-panel labels (the `distill_*` pipeline that built gold) should push toward/past 0.893 —
  this *under*-states the achievable, which is why it justifies the labelling spend.
- **Frozen encoder + linear head, family-level (8 classes), 244-col test.** The production
  build is 250-class, encoder-fine-tuned, and must clear the corpus-honest relocation gate.
  0.82 here is a strong directional signal, not a promotion number.
- The test families are the ones the encoder separates well; the full corpus has more.

## Recommended next phase (the structured investment)

The cheap proof greenlights the spend:

1. **Scale the mining** across all contested families (the corpus has ~570k candidates).
2. **LLM-label a quality set** via the `distill_*` blind-panel pipeline (sanctioned for
   building training data) — clean residual-vs-specific labels on the mined candidates,
   replacing the heuristic. This needs scope/budget sign-off + LLM access; do not fire a
   large labelling job unsupervised.
3. **Feed into the build's ac-01** as header+values training data, then proceed through the
   gates (the corpus-honest relocation gate remains the arbiter).

The prerequisite risk the overnight preview surfaced is now **retired in principle**: the
data lever works. The remaining work is label quality at scale + the production fine-tune.

## GeoNames clean-vocabulary labelling (author input) — validated, with one gap

Replacing the crude header heuristic with **authoritative GeoNames vocabulary membership**
for the geo families (`eval/gold/lens_reference/{cities15000,admin1CodesASCII,iso3166}`):
a mined geo prediction is a positive only if ≥50% of its values are in the vocab, else it's
an over-emission → residual. This correctly handles the contested cases (`[PA,TX,NY,CA]` has
few values in the ISO country list → residual, even though `PA` alone is valid).

| family | header-heuristic recall | **GeoNames-membership recall** |
|---|--:|--:|
| country_code | 0.76 | **1.00** |
| city | 1.00 | **1.00** |
| country | 0.91 | **0.91** |
| region | 0.73 | **0.33** ⚠ |
| overall gold acc | 0.820 | 0.820–0.824 |

- **GeoNames is the clean lever for country / country_code / city** — near-perfect recall and
  RESIDUAL precision up to 0.94. No heuristic noise; authoritative labels.
- **`region` is the gap**: the `admin1CodesASCII` *name* list (4,322) doesn't match real
  region-column values (states/provinces/codes), so it mislabelled most mined region columns
  as residual and starved the positive class. Region needs broader vocab — admin1 + the
  ISO-3166-2 subdivision codes (`iso_3166-2` column is in `iso3166.csv`) + US/national state
  codes — or fall back to the header heuristic / LLM for region.
- Overall stays ~0.82 because the geo gains offset the region loss; fixing region vocab should
  push past it.

## The clean-vocab path (reduces the LLM-labelling spend)

The build's contested-residual training data can be built largely from **authoritative
vocabularies**, not LLM labelling:

- **Geo** (country, country_code, city — and region with broader vocab): GeoNames. Generator
  already exists (`scripts/generate_geonames_geography.py`).
- **Person / full_name**: Wikidata — generator already exists
  (`scripts/generate_wikidata_person_columns.py`).
- The membership signal doubles as a **labeller** for mined corpus candidates (in-vocab →
  positive, out-of-vocab → residual) and a **generator** of clean positives.

LLM-labelling (`distill_*`) then narrows to the genuinely ambiguous residual cases that no
vocabulary covers (entity_name, the messy contested tail) — a much smaller, cheaper job than
labelling everything. That is the refined, cheaper investment plan.
</content>
