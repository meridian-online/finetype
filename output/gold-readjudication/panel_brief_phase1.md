# Gold re-adjudication — blind panel (phase 1)

You are an expert data-type adjudicator for FineType, serving on a multi-model panel.
You label each column **blind**: you do NOT know any prior label or any model's guess.
Judge only the header and the sample values.

## Input
Read `output/gold-readjudication/blind_phase1.jsonl` (≈288 lines). Each line:
`{"id":"g0000","header":"...","sample_values":[...]}`.

## Output — full coverage required
Write `output/gold-readjudication/panel_<PANELID>.jsonl` (you will be told your PANELID).
One JSON object per column, EVERY id covered:
`{"id":"g0000","label":"...","confidence":0.0,"reasoning":"...","runner_up":"..."}`
Process in batches (e.g. 50 columns at a time) and APPEND so nothing truncates; at the
end verify your file has one line per input id (≈288). Reply ONLY with a one-line summary:
count labelled + your read on determinability.

## Candidate types (pick ONE id; use `other:<proposed>` only if nothing fits)
- `representation.discrete.categorical` — small bounded REPEATING vocabulary (a dimension/
  enum), no order. Low distinct-count vs rows.
- `representation.discrete.ordinal` — small bounded set WITH inherent order.
- `representation.text.plain_text` — free-form natural-language text, mostly distinct.
- `representation.text.entity_name` — proper names of entities (orgs, products, works,
  brands, place-names-as-names). Often distinct, capitalised.
- `representation.identifier.alphanumeric_id` — opaque record ids / structured codes
  (accession numbers, SKUs), high cardinality, near-unique.
- `identity.person.full_name` — human personal names.
- `geography.location.{city,region,state,country}` — place names at that level.
- `geography.location.country_code` — 2-letter ISO country codes.
- `technology.internet.url` — web links.
- `representation.identifier.uuid` — UUID format.
- `representation.numeric.{integer_number,decimal_number}` — plain numbers.
- Other FineType types are allowed if clearly correct.

`confidence` is calibrated: 0.9+ = obvious, ~0.5 = genuinely could go two ways. The residual
boundary (categorical vs ordinal vs entity_name vs plain_text vs alphanumeric_id) is the
hard part — reason about cardinality (does a bounded vocabulary repeat?), orderedness, and
whether values are proper names vs free text vs opaque ids. Name your second choice in
`runner_up`.
