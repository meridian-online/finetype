# ac-04 — materialise + blend: manufactured corpus in the training shape

Spec `2026-06-07-reference-data-mining-factory`, ac-04. The firewall survivors
(value-level) are regrouped into synthetic COLUMNS in the distilled shape
`prepare_multibranch_data` consumes — `(final_label, sample_values,
column_name)` — at 50 values/column, each column drawing a
realistic header from `get_header_for_type` so the header branch sees variety.

**100 manufactured columns (5,000 values) across 2 types. Categorical columns: 0 (HARD audit: must be zero).**

Firewall-decimated below the 5-distinct column floor (reported, not a failure): `datetime.component.day_of_week`, `datetime.component.month_name`, `datetime.date.abbreviated_month`, `finance.currency.currency_code`, `finance.currency.currency_symbol`, `geography.address.postal_code`, `geography.address.street_name`, `geography.address.street_suffix`, `geography.location.city`, `geography.location.continent`, `geography.location.country`, `geography.location.country_code`, `geography.location.region`, `geography.location.state_code`, `identity.person.blood_type`, `identity.person.gender_code`, `technology.code.locale_code`, `technology.internet.http_method`, `technology.internet.top_level_domain`. These closed-vocab types lose their full vocabulary to the eval holdout (ac-03) — they were never the starvation problem and the base corpus covers them. Manufacturing's load-bearing contribution is the high-cardinality rare-value diversity, which survives in volume.

## Blend recipe (v19 recipe, seed 42)

The manufactured columns are concatenated onto the base distilled corpus into
one augmented CSV. The PER-TYPE CAP is applied by `prepare_multibranch_data
--distilled-cap 600` at FTMB time (ac-05) — not here — so a
high-volume type (latitude ~460 cols) cannot swamp the base distribution. The
v19 recipe blends distilled:synthetic at `--ratio-distilled 0.5
--samples-per-type 1200 --seed 42`.

Base: `output/distillation-v3/sherlock_distilled.csv.gz` (102,461 columns) + 100 manufactured = 102,561 columns -> `output/mining-factory/coord-only/sherlock_distilled_coords.csv.gz`.

## Per-type manufactured volumes (pre-cap)

| type | distinct values | columns | values in columns |
|---|---:|---:|---:|
| `geography.coordinate.latitude` | 21,436 | 50 | 2,500 |
| `geography.coordinate.longitude` | 29,137 | 50 | 2,500 |
