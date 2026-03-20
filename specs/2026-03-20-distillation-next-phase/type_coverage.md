# Tier 2 Benchmark — Type Coverage Analysis

**Date:** 2026-03-20
**Source:** output/distillation-v3/sherlock_distilled.csv.gz (85,194 rows)
**Filter:** Rows with final_label + parseable sample_values with ≥5 elements

## Summary

| Category | Types | Benchmark columns | Source |
|----------|-------|-------------------|--------|
| Fully distilled (≥10 qualifying rows) | 62 | 620 | Distilled only |
| Partial (1–9 qualifying rows) | 60 | ~330 distilled + ~270 synthetic | Mixed |
| Fully synthetic (0 qualifying rows) | 128 | 1,280 | Generator only |
| **Total** | **250** | **~2,500** | |

## Exclusions

| Reason | Rows excluded |
|--------|---------------|
| Empty final_label | 8 |
| JSON parse error in sample_values | 599 |
| <5 values in sample_values | 26,163 |
| **Total excluded** | **26,770** |
| **Qualifying rows** | **58,424** |

Note: The 26K rows with <5 values are predominantly single-value or few-value columns
from Sherlock (headerless tabular data). These provide weak classification signal and
are excluded from the benchmark. They remain available for training data (Spec 2).

## Fully Distilled Types (62)

Types with ≥10 qualifying rows — all 10 benchmark columns drawn from distilled data.

| Type | Agree | Disagree | Total |
|------|-------|----------|-------|
| representation.discrete.categorical | 937 | 9,894 | 10,831 |
| representation.text.entity_name | 5,619 | 4,705 | 10,324 |
| representation.text.plain_text | 17 | 4,696 | 4,713 |
| identity.person.full_name | 573 | 2,879 | 3,452 |
| representation.numeric.integer_number | 442 | 2,856 | 3,298 |
| geography.location.region | 846 | 2,418 | 3,264 |
| geography.location.city | 44 | 2,248 | 2,292 |
| representation.identifier.alphanumeric_id | 493 | 1,696 | 2,189 |
| geography.location.country | 1,634 | 473 | 2,107 |
| representation.numeric.decimal_number | 1,419 | 112 | 1,531 |
| datetime.component.day_of_week | 886 | 585 | 1,471 |
| representation.text.sentence | 154 | 1,199 | 1,353 |
| representation.text.word | 15 | 1,241 | 1,256 |
| datetime.component.year | 994 | 160 | 1,154 |
| identity.person.gender_code | 99 | 944 | 1,043 |
| representation.identifier.increment | 4 | 975 | 979 |
| geography.address.full_address | 528 | 259 | 787 |
| identity.person.gender | 302 | 368 | 670 |
| identity.commerce.isbn | 20 | 588 | 608 |
| geography.location.country_code | 108 | 326 | 434 |
| datetime.time.hm_24h | 15 | 431 | 446 |
| geography.address.street_name | 2 | 333 | 335 |
| representation.identifier.numeric_code | 63 | 224 | 287 |
| representation.discrete.ordinal | 53 | 224 | 277 |
| representation.text.paragraph | 98 | 153 | 251 |
| identity.person.username | 0 | 242 | 242 |
| identity.person.last_name | 41 | 115 | 156 |
| representation.numeric.si_number | 2 | 147 | 149 |
| representation.boolean.initials | 52 | 180 | 232 |
| representation.boolean.binary | 88 | 56 | 144 |
| technology.code.locale_code | 70 | 73 | 143 |
| finance.currency.currency_code | 50 | 80 | 130 |
| representation.numeric.decimal_number_comma | 37 | 90 | 127 |
| representation.boolean.terms | 12 | 106 | 118 |
| datetime.date.abbreviated_month | 70 | 45 | 115 |
| representation.file.file_size | 59 | 52 | 111 |
| identity.person.first_name | 0 | 105 | 105 |
| representation.numeric.percentage | 48 | 39 | 87 |
| container.array.comma_separated | 0 | 84 | 84 |
| technology.internet.hostname | 65 | 10 | 75 |
| datetime.date.long_full_month | 64 | 0 | 64 |
| representation.file.mime_type | 48 | 3 | 51 |
| geography.transportation.iata_code | 13 | 33 | 46 |
| representation.scientific.measurement_unit | 4 | 41 | 45 |
| representation.file.extension | 5 | 28 | 33 |
| datetime.date.mdy_slash | 28 | 2 | 30 |
| datetime.date.dmy_space_abbrev | 12 | 17 | 29 |
| datetime.date.weekday_abbreviated_month | 2 | 25 | 27 |
| geography.transportation.icao_code | 9 | 18 | 27 |
| finance.currency.amount_comma | 0 | 24 | 24 |
| datetime.time.hms_24h | 17 | 6 | 23 |
| datetime.date.iso | 22 | 0 | 22 |
| identity.person.weight | 9 | 27 | 36 |
| geography.address.street_suffix | 3 | 15 | 18 |
| geography.location.continent | 106 | 96 | 202 |
| geography.address.postal_code | 1 | 15 | 16 |
| datetime.period.fiscal_year | 1 | 16 | 17 |
| finance.currency.amount | 0 | 15 | 15 |
| finance.currency.amount_nodecimal | 1 | 12 | 13 |
| container.array.semicolon_separated | 1 | 9 | 10 |
| geography.coordinate.dms | 0 | 10 | 10 |
| technology.development.version | 0 | 10 | 10 |

## Notable Patterns

- **entity_name** (10,324) and **categorical** (10,831) dominate — these are Sherlock's most common types
- **username** (242), **first_name** (105): 100% disagreement — FineType never classifies these correctly
- **plain_text** (4,713): 99.6% disagreement — FineType over-classifies as entity_name
- **increment** (979): 99.6% disagreement — FineType misclassifies sequential IDs
- **city** (2,292): 98% disagreement — FineType misses geographic entities without headers
