# Gold re-adjudication — blind panel (phase 2: header-heuristic + datetime re-run)

You are an expert data-type adjudicator for FineType, on a multi-model panel. Label each
column **blind** — you do NOT know any prior label or any model's guess. Judge only the
header and the sample values.

## Input
Read `output/gold-readjudication/blind_phase2.jsonl` (~220 lines):
`{"id":"p0000","header":"...","sample_values":[...]}`.

## Output — full coverage
Write `output/gold-readjudication/panel2_<PANELID>.jsonl` (you'll be told PANELID). One JSON
object per column, EVERY id covered:
`{"id":"p0000","label":"...","confidence":0.0,"reasoning":"...","runner_up":"..."}`
Batch in ~50s and APPEND so nothing truncates; verify one line per input id (~220) at the
end. Reply ONLY a one-line summary.

## IMPORTANT — use these EXACT canonical type ids (do not invent variants)

**Datetime (pick the precise sub-format; do NOT use `calendar_date`/`date`/`datetime`):**
- `datetime.date.iso` (YYYY-MM-DD) · `datetime.date.dmy_slash` · `datetime.date.mdy_slash`
- `datetime.timestamp.iso_8601` (date+T+time) · `datetime.timestamp.sql_standard` (YYYY-MM-DD HH:MM:SS)
- `datetime.epoch.unix_seconds` (10-digit) · `datetime.epoch.unix_milliseconds` (13-digit)
- `datetime.component.year` (bare 4-digit year) · `datetime.offset.utc` · `datetime.time.iso`
- If a date's slash order is ambiguous, prefer `datetime.date.iso` only for YYYY-MM-DD; else
  pick dmy/mdy by the values; if truly unsure between two real sub-formats, lower confidence.

**Geography:** `geography.coordinate.latitude` (−90..90) · `geography.coordinate.longitude`
(−180..180) · `geography.location.city` / `.region` / `.state` / `.state_code` / `.country`
/ `.country_code` (2-letter ISO).

**Residual / text / id (the hard boundary):**
- `representation.discrete.categorical` (small bounded REPEATING vocabulary, no order)
- `representation.discrete.ordinal` (small bounded set WITH order)
- `representation.text.plain_text` (free-form text, mostly distinct)
- `representation.text.entity_name` (proper names: orgs/products/works/brands)
- `representation.identifier.alphanumeric_id` (opaque ids, high cardinality, near-unique)
- `identity.person.full_name` · `technology.internet.url` · `technology.internet.top_level_domain`
- `representation.identifier.uuid` · `representation.numeric.integer_number` /
  `representation.numeric.decimal_number`

Other canonical FineType types are allowed when clearly correct; use `other:<proposed>` only
if nothing fits. `confidence`: 0.9+ obvious, ~0.5 genuine two-way. Name your second choice in
`runner_up`.
