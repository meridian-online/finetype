# Gold corpus sizing memo

**Spec:** `2026-06-10-human-verified-gold-corpus` ac-01 (gated — author reviews before any build spend)
**Date:** 2026-06-10
**Decision requested:** approve the quotas and the adjudication budget below, or adjust tiers.

## What resolution do we actually need?

The campaign's real decisions hinged on differences of 10–30 percentage points, not 1–2:
latitude precision 0.714 vs ~0.95 (gold anchor), url recall 0.925 vs 0.340 (header-hint
ablation), country recall −31.5% (v22). So the target is: **detect a ≥10pp difference in
per-type precision/recall at 95% confidence**, not maximal tightness. That is what sets
the per-type n.

95% CI half-width on a proportion, by sample size:

| n per type | worst case (p=0.5) | typical (p=0.9) |
|-----------|--------------------|------------------|
| 50        | ±13.9pp            | ±8.3pp           |
| 80        | ±11.0pp            | ±6.6pp           |
| 90        | ±10.3pp            | ±6.2pp           |
| 100       | ±9.8pp             | ±5.9pp           |

n = 90 per contested type resolves every decision the campaign actually faced. The
original spec text said "≤5pp" — that was too aggressive; it would need n ≈ 250–385 per
type and blow the adjudication budget threefold for resolution no past decision needed.
This memo supersedes it with the ≥10pp-detection target.

## Quotas

**Tier 1 — campaign battlegrounds (9 types × 90 = 810 columns).** Each was the subject of
at least one promotion verdict:

| type | why it is contested |
|------|---------------------|
| geography.coordinate.latitude | the C-family fight: v19 emits it on feature floats (gpa, HitRate, mag); latdec relocated rather than fixed it |
| geography.coordinate.longitude | same family, ±180 boundary |
| technology.web.url | header-hint load-bearing (recall 0.925→0.340 without hints) |
| datetime.offset.utc_offset | utc-on-integers over-emit; positives ~absent in GitTables (external sourcing mandatory) |
| geography.location.country_code | B-family + v22's −31.5%; team/exchange-code impostors |
| geography.location.city | v23's collateral damage target (~48k pulled into categorical) |
| geography.location.region | v22 −12.8% |
| representation.discrete.categorical | the absorber — where every bad retrain dumps its over-emit |
| representation.identifier.alphanumeric_id | A-family: msg_id/nct_id vs tight codes (iso6346, mgrs, hash) |

**Tier 2 — model-gap families (7 types × 50 = 350 columns).** Named by choice 0094 /
the header-hint ablation as model-uncovered; decisions on them are coarser (keep/defer a
hint arm), so ±8–14pp suffices:

datetime.component.year (vs 4-digit integer), datetime.timestamp.unix_seconds (collapse
case in the ablation), representation.identifier.isbn, geography.address.postal_code,
finance.money.amount, technology.web.data_uri (8.8× over-emit case), technology.web.tld
(v19 over-emitted 87k columns).

**Backbone — common-type guardrail (4 types × 60 = 240 columns).** integer_number,
decimal_number, plain_text, one common date format. Purpose: a bet that damages the
common types must show up on gold directly, not only via the corpus-honest gate.

**Total: 1,400 columns.**

## What already exists folds in as the seed

- 240 anchor columns (`eval/gold/gold_eval_anchor.tsv`) — already human-curated across
  families A–D; they count toward the Tier 1/2 quotas they overlap.
- ~120-row rare-type review sample (`rare_type_gold_review.py` output) — enters the
  adjudication queue directly (closes open task t-0001692818b74c8d50b76340).

**Net new columns to source: ~1,040.**

## Source split

~70% GitTables via the existing `build_stratified_sample.py` machinery (fixed file list,
seed-reproducible, comparable to all instrument history). ~30% external open data
(data.gov / Kaggle-class), targeted where GitTables is thin or absent: utc_offset
positives (mandatory — the corpus has essentially none), isbn, postal_code (locale
diversity per the Precision Principle), amount. Every external column passes the
choice-0055 realism pre-screen and registers in `eval/datasets/sources.yaml` with
role=gold.

**Why GeoNames and CLDR are not gold column sources.** Both are already in the toolset —
on the other side of the firewall. GeoNames generated v21's geography training columns
and both are top-ranked ingestion sources for the reference-data mining factory (open
spec, `reference_data_inventory.md`). The gold-anchor independence contract exists
precisely to bar gold labels from deriving from "any authoritative source the mining
factory ingests" — otherwise a Sense model trained on factory-mined GeoNames labels
would pass a GeoNames-derived gold set by construction, telling us nothing. They also
fail the realism screen as *columns*: they are canonical reference vocabularies, and the
v21 post-mortem recorded that real-world city columns are messier than GeoNames-derived
ones — the mess is what gold must capture. **Where they do serve: as evidence inside the
labelling lenses** (set-membership checks — is this value a real GeoNames place, a CLDR
month vocabulary?), under the rule below that keeps the human, not the reference set,
as the label source.

## Lineage and reproducibility

Every gold column carries a verifiable chain back to bytes on disk, reusing the
dataset-provenance registry (card 0018, `scripts/dataset_register.py` /
`dataset_verify.py`):

- **GitTables share** — content-addressed already: the fixture keys on
  `(file_content_sha256, column_name)`, the sample is drawn from a committed fixed file
  list, and the corpus is a local snapshot. Rebuild is deterministic today.
- **External share** — every source is downloaded once, vendored locally, and registered
  (snapshot JSON under `eval/datasets/snapshots/` with per-file SHA256 + size +
  fetched-date + licence; `sources.yaml` entry with role=gold). The fixture references
  the registered snapshot, never a URL — re-scoring and rebuilding require **no network
  and survive source-site drift**. `dataset_verify.py` re-hashes before any run that
  consumes gold.
- **Labels** — `labeller` + `provenance` columns (anchor convention) record which lens
  votes and/or which human verdict produced each label, so any label is auditable to
  its evidence.

## Labelling and the adjudication budget

Independence contract (inherited from spec 2026-06-05-gold-eval-anchor ac-01): no gold
label may derive from YDF, the Sense cascade, or mining-factory sources. Pre-labelling
lenses: (1) value-validator (taxonomy JSON-Schema pass rates), (2) header-semantic,
(3) external-reference (set-membership against GeoNames, CLDR, ISO code lists) /
model-free heuristic. Unanimous agreement → consensus label; any disagreement or low
confidence → the author's queue. **Decisive-lens rule:** because GeoNames/CLDR are also
mining-factory ingestion sources, the external-reference lens may corroborate but never
decide alone — any column where it is the tie-breaking vote routes to the human queue,
so the independence contract holds (the human is the label source; reference sets are
evidence).

Expected queue: 15–25% of 1,400 ≈ **210–350 columns**. At 30–40 seconds per verdict
(the `rare_type_gold_review.py` format: header + sample values + correct/wrong/unsure)
that is **2–3.5 hours, hard-capped at 350 columns**. If the queue overflows the cap,
Tier 2 quotas trim first (50 → 35) — Tier 1 resolution is not negotiable.

## What this buys

- Per-type precision/recall with ±6–10pp CIs on every type the campaign has fought over
  — the headline becomes "FineType is right N% of the time on human-checked data".
- The instrument audit (ac-07): gated-YDF, the scoreboard, and the gate bands each get a
  measured error rate instead of a suspicion.
- Every future deletion/simplification gets a single regression gate (card 0019
  scenario 5) — with the recorded exception that the corpus-honest gate's blocking role
  survives unless gold provably catches relocation.

## Open question at sign-off

Whether 90/50/60 per-type quotas should flex by observed corpus frequency (e.g. city is
abundant, utc_offset is not) — proposal: keep flat quotas for CI comparability and let
sourcing difficulty, not frequency, drive any exception, recorded per-type in the
manifest's provenance column.
