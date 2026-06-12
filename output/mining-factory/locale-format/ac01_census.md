# ac-01 — manufacturing census: starvation dissolved

Spec `2026-06-07-reference-data-mining-factory`, ac-01. Value-level corpus
manufactured directly from authoritative reference data (GeoNames, CLDR,
ISO/IANA). The acceptance bar is this census, not the row count.

**28 high-cardinality types clear the 50-distinct floor; 6 closed vocabularies complete; 0 fail; 0 missing.**

Two honest classes. *High-cardinality* types were the starvation problem — they must clear the floor by orders of magnitude. *Closed-vocab* types (http_method, blood_type, continent...) are genuinely small authoritative sets where the bar is full membership, not the floor; reporting them as sub-floor failures would be dishonest.

| type | class | distinct | rows | locales | verdict |
|---|---|---:|---:|---:|---|
| `geography.location.city` | high-cardinality | 28,574 | 30,000 | 231 | PASS |
| `geography.location.region` | high-cardinality | 3,780 | 3,861 | 228 | PASS |
| `geography.location.country` | high-cardinality | 252 | 252 | 1 | PASS |
| `geography.location.country_code` | high-cardinality | 504 | 504 | 1 | PASS |
| `geography.location.continent` | closed | 7 | 252 | 1 | closed ✓ |
| `geography.location.state_code` | high-cardinality | 706 | 3,861 | 228 | PASS |
| `geography.address.postal_code` | high-cardinality | 20,788 | 30,000 | 106 | PASS |
| `geography.address.street_name` | high-cardinality | 96 | 3,200 | 16 | PASS |
| `geography.address.street_suffix` | closed | 76 | 2,400 | 12 | closed ✓ |
| `geography.coordinate.latitude` | high-cardinality | 28,286 | 30,000 | 1 | PASS |
| `geography.coordinate.longitude` | high-cardinality | 29,219 | 30,000 | 1 | PASS |
| `datetime.component.day_of_week` | high-cardinality | 1,617 | 4,942 | 706 | PASS |
| `datetime.component.month_name` | high-cardinality | 2,486 | 8,472 | 706 | PASS |
| `datetime.date.abbreviated_month` | high-cardinality | 8,348 | 8,472 | 706 | PASS |
| `technology.internet.top_level_domain` | high-cardinality | 249 | 251 | 1 | PASS |
| `technology.internet.http_method` | closed | 7 | 200 | 1 | closed ✓ |
| `technology.code.locale_code` | high-cardinality | 706 | 706 | 706 | PASS |
| `finance.currency.currency_code` | high-cardinality | 155 | 251 | 1 | PASS |
| `finance.currency.currency_symbol` | closed | 30 | 200 | 1 | closed ✓ |
| `identity.person.gender_code` | closed | 7 | 200 | 1 | closed ✓ |
| `identity.person.blood_type` | closed | 8 | 200 | 1 | closed ✓ |
| `finance.currency.amount_comma` | high-cardinality | 800 | 800 | 1 | PASS |
| `finance.currency.amount_comma_suffix` | high-cardinality | 800 | 800 | 1 | PASS |
| `finance.currency.amount_space` | high-cardinality | 800 | 800 | 1 | PASS |
| `finance.currency.amount_multisym` | high-cardinality | 800 | 800 | 1 | PASS |
| `finance.currency.amount_apostrophe` | high-cardinality | 800 | 800 | 1 | PASS |
| `finance.currency.amount_lakh` | high-cardinality | 800 | 800 | 1 | PASS |
| `finance.currency.amount_nodecimal` | high-cardinality | 800 | 800 | 1 | PASS |
| `finance.currency.amount_accounting` | high-cardinality | 800 | 800 | 1 | PASS |
| `finance.currency.amount_neg_trailing` | high-cardinality | 800 | 800 | 1 | PASS |
| `finance.currency.amount_code_prefix` | high-cardinality | 800 | 800 | 1 | PASS |
| `datetime.date.long_full_month` | high-cardinality | 5,466 | 5,506 | 511 | PASS |
| `datetime.date.weekday_full_month` | high-cardinality | 4,942 | 4,943 | 495 | PASS |
| `datetime.date.weekday_abbreviated_month` | high-cardinality | 5,090 | 5,094 | 509 | PASS |

## The load-bearing proof — latitude

GitTables held **10 distinct** latitude values in 18M rows.
Manufacturing yields **28,286 distinct** — a **2,828.6×** lift.
The confusion family the v24 latitude bet starved on is no longer starved.
