# ac-04 — materialise + blend: manufactured corpus in the training shape

Spec `2026-06-07-reference-data-mining-factory`, ac-04. The firewall survivors
(value-level) are regrouped into synthetic COLUMNS in the distilled shape
`prepare_multibranch_data` consumes — `(final_label, sample_values,
column_name)` — at 50 values/column, each column drawing a
realistic header from `get_header_for_type` so the header branch sees variety.

**335 manufactured columns (16,445 values) across 18 types. Categorical columns: 0 (HARD audit: must be zero).**

Firewall-decimated below the 5-distinct column floor (reported, not a failure): `geography.location.continent`, `identity.person.blood_type`, `technology.internet.http_method`. These closed-vocab types lose their full vocabulary to the eval holdout (ac-03) — they were never the starvation problem and the base corpus covers them. Manufacturing's load-bearing contribution is the high-cardinality rare-value diversity, which survives in volume.

## Blend recipe (v19 recipe, seed 42)

The manufactured columns are concatenated onto the base distilled corpus into
one augmented CSV. The PER-TYPE CAP is applied by `prepare_multibranch_data
--distilled-cap 600` at FTMB time (ac-05) — not here — so a
high-volume type (latitude ~460 cols) cannot swamp the base distribution. The
v19 recipe blends distilled:synthetic at `--ratio-distilled 0.5
--samples-per-type 1200 --seed 42`.

Base: `output/distillation-v3/sherlock_distilled.csv.gz` (102,461 columns) + 335 manufactured = 102,796 columns -> `output/mining-factory/sherlock_distilled_mfg.csv.gz`.

## Per-type manufactured volumes (pre-cap)

| type | distinct values | columns | values in columns |
|---|---:|---:|---:|
| `datetime.component.day_of_week` | 6 | 1 | 6 |
| `datetime.component.month_name` | 6 | 1 | 6 |
| `datetime.date.abbreviated_month` | 3,699 | 50 | 2,500 |
| `finance.currency.currency_code` | 149 | 3 | 149 |
| `finance.currency.currency_symbol` | 29 | 1 | 29 |
| `geography.address.postal_code` | 16,954 | 50 | 2,500 |
| `geography.address.street_name` | 92 | 2 | 92 |
| `geography.address.street_suffix` | 11 | 1 | 11 |
| `geography.coordinate.latitude` | 21,478 | 50 | 2,500 |
| `geography.coordinate.longitude` | 29,140 | 50 | 2,500 |
| `geography.location.city` | 27,437 | 50 | 2,500 |
| `geography.location.continent` | 1 | 0 | 0 |
| `geography.location.country` | 18 | 1 | 18 |
| `geography.location.country_code` | 224 | 5 | 224 |
| `geography.location.region` | 3,764 | 50 | 2,500 |
| `geography.location.state_code` | 56 | 2 | 56 |
| `identity.person.blood_type` | 2 | 0 | 0 |
| `identity.person.gender_code` | 5 | 1 | 5 |
| `technology.code.locale_code` | 699 | 14 | 699 |
| `technology.internet.http_method` | 1 | 0 | 0 |
| `technology.internet.top_level_domain` | 152 | 3 | 150 |
