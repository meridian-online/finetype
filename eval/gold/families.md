# Confusion-family scope (fixed)

The gold set covers four confusion families — the columns whose true type is
*contested between lenses*. Each family pairs a **contested label** (what a lens
mis-predicts) with the **true labels** the curated oracle assigns. The gold set
is the tie-breaker. Scope is fixed here (spec ac-02); the curation script
`curate_gold_anchor.py` enumerates the same four families in code.

Evidence for families A and B: memories `v23-ac01-finding`, `v23-ac08-outcome`,
and `eval/gittables/corpus_pass/report.md`. Families C and D are the shared-shape
numerics the mining factory cannot disambiguate from value membership alone
(roadmap Line B "Honest scope").

---

## Family A — tight-code vs alphanumeric-id

**Contested label(s):** `geography.transportation.iso6346`, `geography.coordinate.mgrs`
**True labels in family:** `representation.identifier.alphanumeric_id` (the
dominant truth), and genuine `iso6346` / `mgrs` *iff* any real ones exist.

YDF mislabels `msg_id` columns as `iso6346` (3,388 cols; 2,828 prefixed `msg`)
and `stock_id` / `cluster_id` / package-code columns as `mgrs`. Per
`v23-ac01-finding`, **zero** of these match the tight patterns — so the truth is
almost entirely `alphanumeric_id`.

**Oracle tie-breaker:** a value is `iso6346` only if it matches
`^[A-Z]{3}[UJZ]\d{7}$`; `mgrs` only if it matches the MGRS grid pattern
`^\d{1,2}[C-X][A-Z]{2}\d+$`. If **no** values match the tight pattern, the true
label is `alphanumeric_id`.

## Family B — country_code vs categorical code

**Contested label:** `geography.location.country_code`
**True labels in family:** `geography.location.country_code` (real ISO 3166-1
alpha-2), `representation.discrete.categorical` (3-letter team abbreviations,
exchange codes).

YDF labels 4,038 columns `country_code`; many are 3-letter sports team
abbreviations (`UTA`, `FLA` — header `TEAM_ABBREVIATION`) or 3-letter exchange
codes (`GER` — header `exchange`), which are 3 chars, not the 2 of alpha-2.

**Oracle tie-breaker:** `country_code` only if the values are 2-letter codes
drawn from the ISO 3166-1 alpha-2 set *and* the header reads geographic
(`country`, `nation`, `iso`, `cc`). 3-letter team/exchange codes →
`categorical`. (The alpha-2 membership check here is the oracle's own
hand-authored set used for *disambiguation*, not a B2 sieve vocabulary; columns
that would need a full Geonames lookup to label are curated by hand instead — see
README independence contract.)

## Family C — latitude vs longitude vs temperature

**Contested label(s):** `geography.coordinate.latitude`, `geography.coordinate.longitude`
**True labels in family:** `geography.coordinate.latitude`,
`geography.coordinate.longitude`, `representation.numeric.decimal_number`
(temperature, scores, ratios — the taxonomy has **no** temperature type, so a
bare temperature column's true label is `decimal_number`).

All three are signed/unsigned floats of similar shape; value membership alone
cannot separate them. This is where the late-fusion model's header + sibling
context earns its keep, and exactly what the gold set must measure.

**Oracle tie-breaker:** header token + value range. `lat`/`latitude` with values
in [-90, 90] → `latitude`; `lon`/`lng`/`longitude` in [-180, 180] → `longitude`;
a float column with a non-geographic header (temperature, score, rate, index) →
`decimal_number`.

## Family D — component.year vs integer_number

**Contested label:** `datetime.component.year`
**True labels in family:** `datetime.component.year`,
`representation.numeric.integer_number`.

4-digit integers are ambiguous: a real year vs a count, an id, or a 4-digit code.

**Oracle tie-breaker:** `component.year` only if the header reads temporal
(`year`, `yr`, `season`) *and* values cluster within a plausible year range
(~1500–2100). A 4-digit-integer column with a non-temporal header (counts, ids,
codes) → `integer_number`.

---

## Per-family floor

Target **≥ 20 curated columns per family** (spec ac-03). Families A and B are
abundant (thousands of candidates). Family C's `latitude`/`longitude` *true*
positives are scarcer; the genuine-`iso6346`/`mgrs` true positives in Family A
are essentially absent (per `v23-ac01-finding`). Where a sub-label cannot reach
20, the achieved N and rarity rationale are recorded in the fixture provenance,
not silently dropped.
