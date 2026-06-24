# ac-01 — the authoritative label → Frictionless type/format fold

The map is **data in the taxonomy YAML** (`frictionless: {type, format}` on every
leaf), not an emitter heuristic. This note records the per-leaf review rules and
the honest narrowings, so the fold is auditable and reproducible.

## How the 244 → 16 fold was decided

Primary signal is the existing `broad_type` (the DuckDB cast contract), refined by
domain/category/leaf. Final distribution (244 leaves):

| Frictionless type | count | source |
|---|---|---|
| string | 148 | VARCHAR + every formatted code/identifier |
| date | 34 | datetime.date.* (parseable) |
| datetime | 28 | datetime.timestamp.* |
| number | 11 | DOUBLE/DECIMAL (coords, bare amounts, decimals) |
| integer | 8 | BIGINT/SMALLINT (counts, epochs, increments) |
| time | 5 | datetime.time.* |
| boolean | 3 | BOOLEAN |
| object/array/list | 1/1/1 | container.object.json / .json_array / .comma_separated |
| year/yearmonth/duration/geopoint | 1 each | component.year / date.year_month / duration.iso_8601 / coordinate.coordinates |

## Precision wins (richer than today's regex-only validation)

- `identity.person.email` → `string` / **`email`**
- `representation.identifier.uuid` → `string` / **`uuid`**
- `technology.internet.{url,urn,data_uri}` → `string` / **`uri`**
- **Temporal `format` carries FineType's exact strptime pattern** (e.g.
  `datetime.date.dmy_slash` → `date` / `"%d/%m/%Y"`) — FineType's best signal, not
  a blanket `default`.

## Honest narrowings (deferred, not hidden)

1. **strptime dialect.** Patterns are emitted verbatim where they are
   Python-strptime-portable. The two DuckDB-only codes in use — `%g` (millis, 4
   leaves) and `%-d` (no-pad, 2 leaves) — are **not** portable, so those leaves
   use `format: any` (heuristic parse) rather than a pattern a Frictionless
   consumer would choke on. A follow-up could translate the dialect and restore
   exact patterns.
2. **Exotic calendars → `string`.** `jp_era_*`, `chinese_ymd`, `korean_ymd`,
   `julian`, `period.quarter` have no Frictionless-parseable date form; they map to
   `string` and rely on `x-finetype-label` to carry the real meaning.
3. **Delimited containers.** Only `comma_separated` → `list` (default delimiter
   matches). `pipe/semicolon/whitespace_separated` stay `string` until the
   `delimiter` field-property is emitted — mapping them to `list` now would imply a
   comma delimiter and silently mis-split.
4. **Coordinates.** `latitude`/`longitude` → `number` (separate columns;
   single-column `geopoint` pairing is out of scope). `coordinate.coordinates`
   (already a pair) → `geopoint`; axis-order vs Frictionless `"lon, lat"` is an
   open caveat (choice 0105).
5. **Field-level properties deferred.** `bareNumber`/`decimalChar`/`groupChar`
   (formatted amounts), `delimiter`/`itemType` (lists), `categoriesOrdered`
   (ordinal), `trueValues`/`falseValues`, `missingValues` are field properties, not
   type/format — out of ac-01 scope.

## Gate

`finetype check` validates every definition's block (`Frictionless::validate`):
type ∈ the v2 vocabulary, format legal for that type (temporal types accept any
strptime pattern or `any`). Missing or invalid → non-zero exit. Mirrored by the
`test_every_definition_has_valid_frictionless` unit test. Negative control
verified: corrupting one `type` makes `check` fail with the exact leaf + reason.
