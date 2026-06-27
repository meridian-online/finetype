# m2v-witness specialiser pilot — VERDICT: NO-GO (cheap per-class m2v witness)

Spec 2026-06-27-m2v-witness-specialiser-pilot. Tests the owner's idea: m2v embeddings → ydf,
per single class, as a cheap modular specialiser ("a specialisation framework cheaper than a
feature-model suite across all labels").

## Setup

- Extracted (m2v 1024-dim aggregated embedding, label): gold 931 cols (`gold_labeled.npz`),
  training 131,559 cols / 243 classes (`train_emb.npz`). Reusable extractor:
  `scripts/m2v_witness_extract.py`.
- First cut: one-vs-rest ydf GBT on `representation.text.word` (the hardest semantic residual)
  — AUC 0.838, but precision collapses beyond ~2% recall. Weak standalone veto.
- **Full sweep** (`output/m2v-witness-pilot/sweep_buckets.py`, `sweep_result.md`): one-vs-rest
  GBT for EVERY gold bucket with ≥8 cols (19 buckets), reporting AUC + the abstaining-veto regime
  (max recall at precision ≥0.90). Learner = sklearn HistGBT (15× faster than ydf; cross-checked
  on integer_number: HGB AUC 0.914 ≈ ydf 0.886 — the learner swap preserves the separability signal).

## The sweep (sorted by abstaining-veto recall at precision ≥0.90)

| bucket | kind | AUC | R@P.90 | R@P.95 |
|---|---|---|---|---|
| geography.location.country | **SEMANTIC** | 0.995 | **0.818** | 0.818 |
| datetime.component.year | value | 0.997 | 0.805 | 0.780 |
| geography.location.country_code | value | 0.970 | 0.722 | 0.000 |
| identity.commerce.isbn | value | 0.966 | 0.500 | 0.500 |
| technology.internet.url | value | 0.570 | 0.364 | 0.364 |
| representation.numeric.integer_number | value | 0.914 | 0.145 | 0.119 |
| geography.location.region | SEMANTIC | 0.958 | 0.067 | 0.067 |
| geography.coordinate.latitude | value | 0.446 | 0.026 | 0.026 |
| representation.text.word | SEMANTIC | 0.886 | 0.012 | 0.012 |
| representation.numeric.decimal_number | value | 0.910 | 0.000 | 0.000 |
| representation.identifier.alphanumeric_id | SEMANTIC | 0.806 | 0.000 | 0.000 |
| datetime.date.iso | value | 0.706 | 0.000 | 0.000 |
| geography.coordinate.longitude | value | 0.267 | 0.000 | 0.000 |
| representation.text.plain_text | SEMANTIC | 0.770 | 0.000 | 0.000 |
| geography.location.city | SEMANTIC | 0.987 | 0.000 | 0.000 |
| datetime.epoch.unix_seconds | value | 0.306 | 0.000 | 0.000 |
| datetime.timestamp.sql_standard | value | 0.127 | 0.000 | 0.000 |
| representation.boolean.terms | value | 0.142 | 0.000 | 0.000 |
| representation.text.entity_name | SEMANTIC | 0.959 | 0.000 | 0.000 |

## The load-bearing finding — vocabulary closure, not semantics

What carves a high-precision region is a **bounded value set** (country names ~195, ISO codes,
years, ISBNs). What collapses is an **open vocabulary** (cities, words, entities, free text) —
**even at near-perfect AUC**. `geography.location.city` is the proof: AUC **0.987**, R@P.90 **0.000**.
The embedding *ranks* cities beautifully but the top of the ranking is polluted by adjacent
confusors (country/region/country_code), so no confident sub-region exists.

**AUC is the wrong instrument for an abstaining veto.** city (0.987) and entity_name (0.959) would
pass an AUC screen and ship nothing. Precision-at-recall is the only honest gate.

## Verdict: NO-GO on the cheap per-class m2v-witness framework

- Of the four buckets that carve (country, year, country_code, isbn), **three are already
  value-determined** — Sharpen owns them via validators / closed-sets / checksums. A witness there
  is redundant.
- The **one** semantic bucket the witness cracks is **`country`** (R@P.90 0.818) — and country is a
  **gazetteer lookup**. The deterministic region/country gazetteer reader already on the roadmap
  (Tier-C) gets the same signal more cheaply, auditably, and with **no** corpus-honest relocation
  risk or trained GBT to maintain.
- The **open semantic residual that motivated the witness** (word, plain_text, entity_name,
  alphanumeric_id, city) is **uncrackable** by the cheap m2v witness — exactly the region where it
  would have been the unique lever.

So the cheap per-class m2v witness does not unlock a new capability; it rediscovers the gazetteer
signal on closed-vocab types and is inert on the open residual. Consistent with the frontier
(m2v separability 0.80 < gte 0.89; composed is rule-bound). The proven shippable witness form
(the gte two-sided gate, +1.1pp) used the *stronger* encoder and was still corpus-honest YELLOW.

## Action

1. **Close the pilot NO-GO.** The specialiser framework is the abstaining *value*-rule (already
   shipped) plus the deterministic gazetteer reader — not a trained embedding witness.
2. **Fold `country`/`region` into the gazetteer-reader Tier-C backlog item** (deterministic, no GBT).
3. Did NOT run the gated `word` test — the sweep answers the broader question: the open residual is
   uncrackable, so a two-sided gate on `word` (R@P.90 0.012) has nothing to gate on.
