# Data-asset inventory for the encoder build's training data

**Spec:** 2026-06-18-minilm-encoder-build (ac-01 training-data sourcing)
**Date:** 2026-06-18 · survey of existing repo + `~/datasets` assets

## Headline

**The repo already holds clean/authoritative sources covering essentially the entire
contested-residual training need — including the gaps the overnight proof hit (region, iata).
The build's ac-01 training data can be assembled mostly from existing clean vocabularies +
curated CSVs + generators; LLM-labelling shrinks to a small genuinely-ambiguous tail.**

## CLEAN authoritative sources (membership-labelling + clean positives) — the primary lever

| family | sources | status |
|---|---|---|
| country / country_code | `countries.csv` / `iso3166.csv` (250, name+alpha2+alpha3+region), GeoNames `countryInfo.txt` | clean — already near-perfect (recall 1.00/0.91) |
| city | `world_cities.csv` (33k), GeoNames `cities500.txt` (38M) / `cities15000.txt` | clean — near-perfect (1.00) |
| **region** | **`us_states.csv` (50) + GeoNames `admin1CodesASCII` (4.3k) + `admin2Codes.txt` (47.5k counties) + `world_cities.csv` subcountry** | **FIXES the overnight gap** — admin2 covers the counties/districts gold "region" mostly is |
| **iata_code / airports** | **`eval/datasets/csv/airports.csv`** + GeoNames + `gold-ourairports`/`gold-openflights` snapshots | **fills the gap** (no vocab before) |
| structured codes (isbn/issn/ean/uuid/swift/iban/hex/mime/semver) | **`codes_and_ids.csv`** (clean per-type) | clean positives **AND** the residual-decoy source ("looks like a code") |
| person / full_name / first / last | `people_directory.csv` (100) + Wikidata **Q5** snapshot + `generate_wikidata_person_columns.py` | clean + scalable via generator |
| postal_code | GeoNames `postal/` | available |
| tld / domain | `iana_tlds_alpha.txt` + `gold-majestic-million` snapshot | clean |
| locale / language | CLDR `cldr-46.0.0.json` + GeoNames `iso-languagecodes.txt` | available |
| currency / finance | `financial_data.csv`, `new_finance.csv`, `gold-uk-price-paid` / `gold-nyc-payroll` | available |
| datetime formats | `datetime_formats{,_extended}.csv`, `datetime_coverage.csv` | clean (already strong; deterministic) |

## WEAK corroboration (features / second opinion — NOT clean labels)

- **`dbpedia_annotations.parquet`** (6.2M rows, 800/931 gold coverage): `dbpedia_semantic_class`
  + `dbpedia_similarity` + `schema_semantic_class`. BUT the classes are property *associations*
  (id/type/author/title — largely header-name-driven), not clean type labels. Use as a
  corroboration feature gated by `dbpedia_similarity`, like gated-YDF — not as a labeller.
- gated-YDF (`ydf_prediction` in `corpus_pass`).

## Existing labelled training data (reuse / augment)

- `output/distillation-v3` (6.5M, base Sherlock-distilled — what the overnight preview used)
- `output/distillation-identity` (4M, username/full_name), `distillation-v4`, prior v21-geonames
- `~/datasets/{sherlock, ydf_training}`

## Generators (synthesize clean positives, balanced)

- `generate_geonames_geography.py` (geo, spec 2026-05-24-v21) · `generate_wikidata_person_columns.py`
  (person) · `generate_coverage_closure.py` · `finetype generate` (per-type synthetic) · `fetch_*` for refresh.

## What this changes for ac-01

1. **Region gap closed** — `admin2Codes.txt` (47.5k counties) + `us_states.csv` cover what
   admin1-alone missed (the overnight region recall 0.07–0.33 was a vocab-coverage problem, now
   fixable). iata closed by `airports.csv`.
2. **The residual-decoy negatives are in hand** — `codes_and_ids.csv` is exactly the
   "looks-like-a-specific-type-but-isn't" shape the contested boundary needs.
3. **LLM-labelling (`distill_*`) shrinks to the ambiguous tail** — `entity_name` proper and the
   genuinely-contested residual that no authoritative vocabulary covers. Most of the contested
   mass (geo + person + codes) is clean-vocab addressable.

So the build's data step is largely an **assembly + membership-labelling** job over existing
clean sources, not a from-scratch labelling effort. The corpus-honest gate remains the arbiter.
</content>
