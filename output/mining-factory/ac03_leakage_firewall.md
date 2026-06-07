# ac-03 — leakage firewall: independence from both eval instruments

Spec `2026-06-07-reference-data-mining-factory`, ac-03. Each surviving
manufactured row tested by `eval_leakage.row_hash(header, value)` against the
two instruments the candidate is later judged by, at the scope each demands:

- **Eval holdout (`eval/row_hashes.tsv`) — `(header, value)` row-identity.** The
  corpus-honest gate scores COLUMNS, so leakage is a manufactured row that
  reproduces a holdout `(header, value)`. A `(value, type)` is voided iff
  `row_hash(H, value)` is in the holdout for a synthetic header `H` materialise
  could assign that type (`prepare_multibranch_data` `HEADER_VARIATIONS`/fallback)
  — the exact firewall the training pipeline enforces.
- **Gold anchor (`eval/gold/`) — column-identity + continuous-family values.** The
  independent judge scores held-out COLUMNS. A manufactured corpus carries no
  `(file_content_sha256, column_name)`, so its overlap with the 240
  gold columns is **0** by construction (the project's gold-firewall
  standard, `audit_gold_anchor_leakage.py`). On top of that, the two CONTINUOUS
  confusion families {latitude, longitude} — where memorising an exact coordinate
  is real leakage — are value-voided same-family against the gold columns. Closed
  enums (e.g. country_code) are NOT value-voided: their vocabulary is finite and
  the judge necessarily reuses it, so held-out values don't exist and
  column-identity is the only meaningful independence.

**120,810 of 126,049 rows survive. Eval-holdout voided 5,075 `(header,value)` collisions; gold-anchor voided 164 coordinate values; gold column-identity overlap 0.**

Closed-vocab and small types (continent, http_method, gender_code, blood_type,
country, month_name) lose most rows to the holdout — their full vocabulary
necessarily appears there under obvious headers. That is expected and harmless:
those types were never the starvation problem (the base corpus + `finetype
generate` already cover them), the pipeline filters these rows regardless, and
manufacturing's load-bearing contribution is the HIGH-CARDINALITY rare-value
diversity (latitude, longitude, city, postal_code, region, street_name,
abbreviated_month, locale_code), which survives at >94%.

| type | in | eval-void | gold-void | survive | survive % |
|---|---:|---:|---:|---:|---:|
| `datetime.component.day_of_week` | 1,319 | 749 | 0 | 570 | 43.2% |
| `datetime.component.month_name` | 1,351 | 1,250 | 0 | 101 | 7.5% |
| `datetime.date.abbreviated_month` | 3,782 | 0 | 0 | 3,782 | 100.0% |
| `finance.currency.currency_code` | 251 | 68 | 0 | 183 | 72.9% |
| `finance.currency.currency_symbol` | 200 | 6 | 0 | 194 | 97.0% |
| `geography.address.postal_code` | 25,850 | 9 | 0 | 25,841 | 100.0% |
| `geography.address.street_name` | 3,036 | 182 | 0 | 2,854 | 94.0% |
| `geography.address.street_suffix` | 1,077 | 406 | 0 | 671 | 62.3% |
| `geography.coordinate.latitude` | 23,411 | 290 | 135 | 22,986 | 98.2% |
| `geography.coordinate.longitude` | 30,000 | 55 | 29 | 29,916 | 99.7% |
| `geography.location.city` | 29,650 | 1,171 | 0 | 28,479 | 96.1% |
| `geography.location.continent` | 252 | 247 | 0 | 5 | 2.0% |
| `geography.location.country` | 251 | 233 | 0 | 18 | 7.2% |
| `geography.location.country_code` | 249 | 25 | 0 | 224 | 90.0% |
| `geography.location.region` | 3,850 | 5 | 0 | 3,845 | 99.9% |
| `geography.location.state_code` | 66 | 4 | 0 | 62 | 93.9% |
| `identity.person.blood_type` | 200 | 144 | 0 | 56 | 28.0% |
| `identity.person.gender_code` | 200 | 57 | 0 | 143 | 71.5% |
| `technology.code.locale_code` | 700 | 1 | 0 | 699 | 99.9% |
| `technology.internet.http_method` | 200 | 173 | 0 | 27 | 13.5% |
| `technology.internet.top_level_domain` | 154 | 0 | 0 | 154 | 100.0% |
