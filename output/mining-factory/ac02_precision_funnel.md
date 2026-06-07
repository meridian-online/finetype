# ac-02 — precision funnel: JSON-Schema validation-as-veto

Spec `2026-06-07-reference-data-mining-factory`, ac-02. Every manufactured
value adjudicated by its label's own JSON-Schema (the 240-dim
`schema_pass` vector from `extract_value_features`). Two stages: self-veto
(drop where the value does not validate as its own label) then collision
resolution (same value under >= 2 labels -> source-authority precedence;
the latitude/longitude overlap is exempt — it is the deliberate signal).

**126,049 of 158,224 rows survive (79.7%). Schema-veto dropped 31,240; collision dropped 935.**

| type | in | schema-veto | collision-drop | survive | survive % |
|---|---:|---:|---:|---:|---:|
| `datetime.component.day_of_week` | 4,942 | 3,528 | 95 | 1,319 | 26.7% |
| `datetime.component.month_name` | 8,472 | 7,003 | 118 | 1,351 | 15.9% |
| `datetime.date.abbreviated_month` | 8,472 | 4,690 | 0 | 3,782 | 44.6% |
| `finance.currency.currency_code` | 251 | 0 | 0 | 251 | 100.0% |
| `finance.currency.currency_symbol` | 200 | 0 | 0 | 200 | 100.0% |
| `geography.address.postal_code` | 30,000 | 4,150 | 0 | 25,850 | 86.2% |
| `geography.address.street_name` | 3,200 | 0 | 164 | 3,036 | 94.9% |
| `geography.address.street_suffix` | 2,400 | 1,323 | 0 | 1,077 | 44.9% |
| `geography.coordinate.latitude` | 30,000 | 6,589 | 0 | 23,411 | 78.0% |
| `geography.coordinate.longitude` | 30,000 | 0 | 0 | 30,000 | 100.0% |
| `geography.location.city` | 30,000 | 0 | 350 | 29,650 | 98.8% |
| `geography.location.continent` | 252 | 0 | 0 | 252 | 100.0% |
| `geography.location.country` | 252 | 0 | 1 | 251 | 99.6% |
| `geography.location.country_code` | 504 | 255 | 0 | 249 | 49.4% |
| `geography.location.region` | 3,861 | 0 | 11 | 3,850 | 99.7% |
| `geography.location.state_code` | 3,861 | 3,696 | 99 | 66 | 1.7% |
| `identity.person.blood_type` | 200 | 0 | 0 | 200 | 100.0% |
| `identity.person.gender_code` | 200 | 0 | 0 | 200 | 100.0% |
| `technology.code.locale_code` | 706 | 6 | 0 | 700 | 99.2% |
| `technology.internet.http_method` | 200 | 0 | 0 | 200 | 100.0% |
| `technology.internet.top_level_domain` | 251 | 0 | 97 | 154 | 61.4% |
