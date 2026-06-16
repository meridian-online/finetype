# Determinability panel — blind column type adjudication

You are an expert data-type adjudicator for FineType. You will label each column
**blind** — you do NOT know any model's guess or any prior label. Judge only the
header and the sample values.

## Input
Read `output/determinability-probe/contested_blind.jsonl`. Each line is one column:
`{"id": "...", "header": "...", "sample_values": [...]}`.

## Your task
For EACH column, choose the single most defensible type from the list below, give a
calibrated `confidence` in [0,1] (how clear-cut the call is — 0.9+ = obvious,
~0.5 = genuinely could go two ways), and a one-line `reasoning`. If two types are
near-equal, name the runner-up in `reasoning`.

## Candidate types (pick ONE id)
- `representation.discrete.categorical` — a SMALL, bounded, repeating set of category
  values (a dimension/enum), no inherent order. Low distinct-count vs rows.
- `representation.discrete.ordinal` — a small bounded set WITH inherent order
  (low/med/high, S/M/L, ratings).
- `representation.text.plain_text` — free-form natural-language text, mostly distinct
  (descriptions, phrases, sentences); not a bounded vocabulary.
- `representation.text.entity_name` — proper names of entities: organisations,
  products, works, brands, place-names-as-names. Often distinct, capitalised.
- `representation.identifier.alphanumeric_id` — opaque record identifiers mixing
  letters+digits or structured codes (accession numbers, SKUs, IDs). High cardinality,
  near-unique.
- `identity.person.full_name` — human personal names specifically.
- `geography.location.city` / `geography.location.region` / `geography.location.state`
  / `geography.location.country` — place names at that admin level.
- `geography.location.country_code` — 2-letter ISO country codes.
- `technology.internet.url` — web links.
- `representation.identifier.uuid` — UUID-format identifiers.
- `representation.numeric.integer_number` / `representation.numeric.decimal_number` —
  plain numbers.
- If NONE fit, use `other:<your-proposed-type>` and explain.

The residual boundary (categorical vs ordinal vs entity_name vs plain_text vs
alphanumeric_id) is the hard part and the point of this exercise — think about
cardinality (does a bounded vocabulary repeat?), orderedness, whether values are
proper names vs free text vs opaque ids.

## Output
Write `output/determinability-probe/panel_PANELID.jsonl` (replace PANELID with the
panel number you are told). One JSON line per column:
`{"id": "...", "label": "...", "confidence": 0.0, "reasoning": "..."}`.
Cover every column in the input. Output ONLY the file; reply with a one-line summary
(how many labelled + your overall read on how determinable these columns were).
