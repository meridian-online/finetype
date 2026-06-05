# Gold eval anchor

A small, curated, held-out ground-truth set for the known confusion families.
It is the independent anchor that keeps model evaluation non-circular once YDF
is repurposed from judge to miner (roadmap `2026-06-05-precision-release-roadmap.md`,
Line B / B1; spec `2026-06-05-gold-eval-anchor`).

## Why this exists

The data factory (B2) trains specialised YDF sieves from authoritative reference
data, runs them over GitTables to mine real-world labelled columns, then trains
the best-ever Sense model on that harvest. If the same YDF lens that mines the
training labels is *also* the eval judge, the comparison is circular — Sense is
trained on YDF-mined labels and scored against YDF, so it passes by construction.

This gold set breaks the circle. Its labels come from neither model under
comparison and share no inputs with the mining factory, so a score against it is
an honest measurement, not a tautology.

## The independence contract (load-bearing — do not weaken)

**A gold label MUST NOT derive from any of:**

1. **The YDF lens** — `ydf_prediction` / `ydf_prediction_gated` in
   `eval/gittables/corpus_pass/columns.parquet`. YDF becomes the *miner*; it
   cannot also be the judge.
2. **The Sense cascade** — `sense_prediction` (the shipped `models/default`
   multi-branch model, or any Sense retrain under comparison). Scoring a model
   against its own predictions measures nothing.
3. **Any authoritative source the B2 mining factory ingests** — Geonames, CLDR,
   ISO/IANA/ITU code lists, the synthetic generators, or any Tier-1/Tier-2 source
   catalogued in `2026-06-05-reference-data-inventory`. If a gold label were
   copied from Geonames and a B2 sieve were trained from Geonames, the "independent"
   test would silently re-import the factory's own training signal.

**Labels come from an oracle that shares no inputs with either model under
comparison.** Here the oracle is **human curation expressed as per-family
labelling judgement**: a curator (or a hand-authored labelling function the
curator audits) examines the column's *header* and its *actual values* and
assigns the true taxonomy type by direct inspection against the type definition —
e.g. a column headed `msg_id` whose values are all `^msg\d+$` is
`representation.identifier.alphanumeric_id`, **not** `geography.transportation.iso6346`,
because zero values match the iso6346 pattern `^[A-Z]{3}[UJZ]\d{7}$`.

### Discovery vs labelling — the permitted use of the lens

Finding *candidate* columns may use any signal, including the YDF/Sense
predictions (that is how we know where the confusion lives). **Discovery is not
labelling.** Once a candidate is found, its gold label is assigned by independent
inspection of header + values and recorded with a rationale. The
`ydf_prediction` / `sense_prediction` of a candidate is retained only as
*context* (what the lens got wrong), never as the curated label.

### What "shares no inputs" means in practice

- The curation labelling functions key on **header tokens + value-pattern
  membership** (regex, range, cardinality). These are hand-authored by the
  curator, not learned from the corpus and not copied from a reference vocabulary.
- A B2 sieve trained from Geonames recognises Geonames *values*; the gold oracle
  for a geography column instead asks "does this column's header and value shape
  make it a coordinate, a country code, or a plain number?" — a judgement, not a
  vocabulary lookup. The two share no training signal.
- Where a labelling function would be indistinguishable from a B2 sieve (a pure
  vocabulary membership test against a source B2 also ingests), that column is
  **curated by hand instead**, and the provenance records `labeller: human`.

## Provenance

Every row carries a `provenance` field recording *how* the label was assigned —
the header observed, the value-pattern evidence (as match counts, never raw
values), and the labeller (`human` or the named labelling function). This makes
every label auditable and the independence contract inspectable per-row, not just
asserted here.

## Sanitisation

The fixture stores **no raw third-party cell values** (consistent with the
corpus-pass artefact discipline and the secret-scanner risk). Columns are
identified by `file_content_sha256` + relativised `file_path` + `column_name`;
evidence in `provenance` is expressed as pattern-match *counts* and header
tokens, not echoed values. The `(file_content_sha256, column_name)` identity is
also the key the leakage guard (spec ac-06) uses to exclude these columns from
any training/mining corpus.

## Files

| File | What it is |
|------|------------|
| `README.md` | This contract (spec ac-01). |
| `gold_eval_anchor.tsv` | The versioned fixture (spec ac-03): `family, file_path, file_content_sha256, column_name, curated_label, ydf_context, sense_context, labeller, provenance`. |
| `curate_gold_anchor.py` | Discovery + independent labelling that materialises the fixture (spec ac-02/ac-03). |
| `families.md` | The fixed confusion-family scope (spec ac-02). |

Scoring lives in `scripts/score_gold_anchor.py` (spec ac-04). The leakage guard
(spec ac-06) is `scripts/gold_anchor_guard.py` — keyed on the
`(file_content_sha256, column_name)` identity, not the value-hash the existing
`train_ydf.py` filter uses (that filter is window-sensitive: re-sampling the
same column changes its hash and slips it past). It is wired into
`scripts/train_ydf.py`'s exclusion path, audited by
`scripts/audit_gold_anchor_leakage.py`, and tested by
`scripts/test_gold_anchor_guard.py` (zero `(file, column)` overlap with the
current training corpus, plus a positive test that the guard drops an injected
gold column). ac-07 is the deferred counterpart: the same identity audit against
the B2 harvested corpus, once that corpus exists.
