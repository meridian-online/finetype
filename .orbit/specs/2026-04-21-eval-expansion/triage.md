# Triage — eval/datasets/manifest.csv (ac-03)

**Status:** DRAFT — deterministic classification from `prescreen_results.tsv`.
Per spec constraint #1 this draft is subject to Hugh's human review before
ac-04 replace-execution begins. Override any row by editing the Action cell
and noting the override rationale in the Notes column.

**Source:** eval/prescreen_results.tsv
**Total rows:** 338

## Known audit-surfaced gaps (NOT in this worklist)

These are **missing** rows, not existing-row replacements, and are handled
under **ac-05 coverage gate** (MADR 0057):

- `finance.institution.swift_bic` — zero coverage
- `identity.medical.cpt` — zero coverage (restricted registry; synthetic-necessary per MADR 0055 carve-out)
- `identity.medical.loinc` — zero coverage (likely synthetic-necessary)
- `technology.format.excel_format` — zero coverage
- `technology.internet.user_agent` — zero coverage

All five are added as new rows under ac-05 coverage closure with their own
source_url / licence / fetched_date populated in `manifest.csv`.

## v16 misclassification rows (NOT replace candidates)

The seven v16 errors in CLAUDE.md (user_agent × 2, geojson, phone/ssn,
id/username, fiscal_year/year, locale/categorical) are model errors, not
eval-realism issues. Per spec constraint #8 — existing gt labels are not
renegotiated — these rows stay at action=keep. Model-accuracy concerns are
out of scope for this programme.

## Summary

```
  replace  0
  augment  0
  keep     338
```

## Decision rule (mechanical)

1. `pass_floors=True` → **keep**
2. fail on entropy+skew only → **keep** (categorical signature)
3. fail on null_rate only → **keep** (legitimate sparsity)
4. multiple failing axes → **augment**
5. pre-screen error / empty column → **replace**

## Worklist

| Dataset | Column | gt_label | Action | Rationale | gt_label_change |
|---|---|---|---|---|---|
| airports | altitude | number | **keep** | pass_floors=True — meets realism floors |  |
| airports | city | city | **keep** | pass_floors=True — meets realism floors |  |
| airports | country | country | **keep** | pass_floors=True — meets realism floors |  |
| airports | iata | iata | **keep** | pass_floors=True — meets realism floors |  |
| airports | icao | icao | **keep** | pass_floors=True — meets realism floors |  |
| airports | id | id | **keep** | pass_floors=True — meets realism floors |  |
| airports | latitude | latitude | **keep** | pass_floors=True — meets realism floors |  |
| airports | longitude | longitude | **keep** | pass_floors=True — meets realism floors |  |
| airports | name | airport name | **keep** | pass_floors=True — meets realism floors |  |
| airports | source | category | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| airports | timezone | time zone | **keep** | pass_floors=True — meets realism floors |  |
| airports | type | category | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| airports | utc_offset | utc offset | **keep** | pass_floors=True — meets realism floors |  |
| api_users_json | address.city | city | **keep** | high-null semi-structured / optional field; real-world sparsity |  |
| api_users_json | address.country | country code | **keep** | high-null semi-structured / optional field; real-world sparsity |  |
| api_users_json | address.postal_code | postal code | **keep** | high-null semi-structured / optional field; real-world sparsity |  |
| api_users_json | email | email | **keep** | pass_floors=True — meets realism floors |  |
| api_users_json | name | name | **keep** | pass_floors=True — meets realism floors |  |
| api_users_json | phone | telephone | **keep** | pass_floors=True — meets realism floors |  |
| api_users_json | profile_url | url | **keep** | pass_floors=True — meets realism floors |  |
| books_catalog | author | author | **keep** | pass_floors=True — meets realism floors |  |
| books_catalog | description | description | **keep** | pass_floors=True — meets realism floors |  |
| books_catalog | isbn | isbn | **keep** | pass_floors=True — meets realism floors |  |
| books_catalog | language | language | **keep** | pass_floors=True — meets realism floors |  |
| books_catalog | pages | number | **keep** | pass_floors=True — meets realism floors |  |
| books_catalog | price_usd | price | **keep** | pass_floors=True — meets realism floors |  |
| books_catalog | publisher | entity name | **keep** | pass_floors=True — meets realism floors |  |
| books_catalog | rating | rating | **keep** | pass_floors=True — meets realism floors |  |
| books_catalog | title | title | **keep** | pass_floors=True — meets realism floors |  |
| books_catalog | url | url | **keep** | pass_floors=True — meets realism floors |  |
| books_catalog | year_published | year | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | credit_card | credit card number | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | ean | ean | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | hex_color | color | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | iban | iban | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | isbn | isbn | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | issn | issn | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | locale | locale code | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | mime_type | file format | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | semantic_version | version | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | sha256 | hash | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | swift_code | swift code | **keep** | pass_floors=True — meets realism floors |  |
| codes_and_ids | uuid | uuid | **keep** | pass_floors=True — meets realism floors |  |
| countries | alpha-2 | country code | **keep** | pass_floors=True — meets realism floors |  |
| countries | alpha-3 | country code | **keep** | pass_floors=True — meets realism floors |  |
| countries | country-code | code | **keep** | pass_floors=True — meets realism floors |  |
| countries | iso_3166-2 | code | **keep** | pass_floors=True — meets realism floors |  |
| countries | name | country | **keep** | pass_floors=True — meets realism floors |  |
| countries | region | region | **keep** | pass_floors=True — meets realism floors |  |
| countries | region-code | code | **keep** | pass_floors=True — meets realism floors |  |
| countries | sub-region | region | **keep** | pass_floors=True — meets realism floors |  |
| countries | sub-region-code | code | **keep** | pass_floors=True — meets realism floors |  |
| covid_timeseries | Confirmed | number | **keep** | pass_floors=True — meets realism floors |  |
| covid_timeseries | Country | country | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| covid_timeseries | Date | iso date | **keep** | pass_floors=True — meets realism floors |  |
| covid_timeseries | Deaths | number | **keep** | pass_floors=True — meets realism floors |  |
| covid_timeseries | Recovered | number | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | clf_timestamp | clf timestamp | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | compact_dmy | compact dmy | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | compact_ymd | compact ymd | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | dmy_dash | dmy dash | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | fiscal_year | fiscal year | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | iso_8601_offset | iso 8601 offset | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | iso_week | iso week | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | mdy_dash | mdy dash | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | quarter | quarter | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | rfc_3339 | rfc 3339 | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | syslog_bsd | syslog bsd | **keep** | pass_floors=True — meets realism floors |  |
| datetime_coverage | ymd_dot | ymd dot | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | day_of_week | day of week | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | duration_iso | duration | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | eu_date | dmy slash | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | iso_date | iso date | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | iso_timestamp | timestamp | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | month_name | month name | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | sql_timestamp | sql timestamp | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | time_24h | time 24h | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | timezone | time zone | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | unix_epoch | time | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | unix_ms | time | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | us_date | mdy slash | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | utc_offset | utc offset | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats | year | year | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats_extended | abbreviated_month_date | abbreviated month date | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats_extended | american_timestamp | american timestamp | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats_extended | eu_dot_date | eu dot date | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats_extended | european_timestamp | european timestamp | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats_extended | long_full_month_date | long full month date | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats_extended | rfc_2822_timestamp | rfc 2822 timestamp | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats_extended | time_12h | time 12h | **keep** | pass_floors=True — meets realism floors |  |
| datetime_formats_extended | time_12h_seconds | time 12h seconds | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | depth | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | depthError | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | dmin | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | gap | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | horizontalError | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | id | alphanumeric id | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | latitude | latitude | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | locationSource | category | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| earthquakes_2024 | longitude | longitude | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | mag | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | magError | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | magNst | number | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | magSource | category | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| earthquakes_2024 | magType | category | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| earthquakes_2024 | net | category | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| earthquakes_2024 | nst | number | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | place | address | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | rms | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | status | status | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| earthquakes_2024 | time | iso timestamp milliseconds | **keep** | pass_floors=True — meets realism floors |  |
| earthquakes_2024 | type | category | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| earthquakes_2024 | updated | iso timestamp milliseconds | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders | credit_card_last4 | code | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders | currency | currency | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders | customer_email | email | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders | is_gift | boolean | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| ecommerce_orders | order_date | iso date | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders | order_id | code | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders | phone | telephone | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders | shipping_country | country | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders | shipping_postal_code | postal code | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders | status | status | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders | total_price | price | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders | tracking_url | url | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders_json | currency | currency code | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| ecommerce_orders_json | customer_email | email | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders_json | is_express | boolean | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| ecommerce_orders_json | order_date | iso timestamp | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders_json | order_id | alphanumeric id | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders_json | product | category | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders_json | quantity | number | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| ecommerce_orders_json | status | status | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders_json | total | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| ecommerce_orders_json | unit_price | price | **keep** | pass_floors=True — meets realism floors |  |
| finance_coverage | bitcoin_address | bitcoin address | **keep** | pass_floors=True — meets realism floors |  |
| finance_coverage | currency_symbol | currency symbol | **keep** | pass_floors=True — meets realism floors |  |
| finance_coverage | cusip | cusip | **keep** | pass_floors=True — meets realism floors |  |
| finance_coverage | isin | isin | **keep** | pass_floors=True — meets realism floors |  |
| finance_coverage | lei | lei | **keep** | pass_floors=True — meets realism floors |  |
| financial_data | close_price | price | **keep** | pass_floors=True — meets realism floors |  |
| financial_data | currency | currency | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| financial_data | date | iso date | **keep** | pass_floors=True — meets realism floors |  |
| financial_data | dividend_yield | percentage | **keep** | pass_floors=True — meets realism floors |  |
| financial_data | exchange | category | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| financial_data | high_price | price | **keep** | pass_floors=True — meets realism floors |  |
| financial_data | low_price | price | **keep** | pass_floors=True — meets realism floors |  |
| financial_data | market_cap | value | **keep** | pass_floors=True — meets realism floors |  |
| financial_data | open_price | price | **keep** | pass_floors=True — meets realism floors |  |
| financial_data | pe_ratio | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| financial_data | ticker | code | **keep** | pass_floors=True — meets realism floors |  |
| financial_data | volume | number | **keep** | pass_floors=True — meets realism floors |  |
| geography_data | city | city | **keep** | pass_floors=True — meets realism floors |  |
| geography_data | coordinates | coordinates | **keep** | pass_floors=True — meets realism floors |  |
| geography_data | country | country | **keep** | pass_floors=True — meets realism floors |  |
| geography_data | country_code | country code | **keep** | pass_floors=True — meets realism floors |  |
| geography_data | elevation_m | number | **keep** | pass_floors=True — meets realism floors |  |
| geography_data | full_address | address | **keep** | pass_floors=True — meets realism floors |  |
| geography_data | latitude | latitude | **keep** | pass_floors=True — meets realism floors |  |
| geography_data | longitude | longitude | **keep** | pass_floors=True — meets realism floors |  |
| geography_data | postal_code | postal code | **keep** | pass_floors=True — meets realism floors |  |
| geography_data | region | region | **keep** | pass_floors=True — meets realism floors |  |
| geography_data | street_number | number | **keep** | pass_floors=True — meets realism floors |  |
| iris | petal_length | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| iris | petal_width | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| iris | sepal_length | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| iris | sepal_width | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| iris | species | category | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | date_of_birth | iso date | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | diagnosis_code | icd10 | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | first_name | first name | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | gender | gender | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| medical_records | heart_rate | number | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | height_in | height | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | is_admitted | boolean | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | last_name | last name | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | npi | npi | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | patient_id | id | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | temperature_f | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | visit_date | iso date | **keep** | pass_floors=True — meets realism floors |  |
| medical_records | weight_lbs | weight | **keep** | pass_floors=True — meets realism floors |  |
| multilingual | address | address | **keep** | pass_floors=True — meets realism floors |  |
| multilingual | country | country | **keep** | pass_floors=True — meets realism floors |  |
| multilingual | date | date | **keep** | pass_floors=True — meets realism floors |  |
| multilingual | locale | language code | **keep** | pass_floors=True — meets realism floors |  |
| multilingual | name | name | **keep** | pass_floors=True — meets realism floors |  |
| multilingual | phone | telephone | **keep** | pass_floors=True — meets realism floors |  |
| multilingual | postal_code | postal code | **keep** | pass_floors=True — meets realism floors |  |
| multilingual | price | price | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | content_type | file format | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | destination_ip | ip_v4 | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | method | http method | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | payload_size_bytes | number | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | query_params | code | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | request_id | uuid | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | response_time_ms | number | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | source_ip | ip_v4 | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | status_code | http status code | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | timestamp | iso timestamp milliseconds | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | url_path | code | **keep** | pass_floors=True — meets realism floors |  |
| network_logs | user_agent | user agent | **keep** | pass_floors=True — meets realism floors |  |
| new_finance | aba_routing | aba routing | **keep** | pass_floors=True — meets realism floors |  |
| new_finance | bsb | bsb | **keep** | pass_floors=True — meets realism floors |  |
| new_finance | figi | figi | **keep** | pass_floors=True — meets realism floors |  |
| new_geography | dms | dms | **keep** | pass_floors=True — meets realism floors |  |
| new_geography | geohash | geohash | **keep** | pass_floors=True — meets realism floors |  |
| new_geography | geojson | geojson | **keep** | pass_floors=True — meets realism floors |  |
| new_geography | h3 | h3 | **keep** | pass_floors=True — meets realism floors |  |
| new_geography | hs_code | hs code | **keep** | pass_floors=True — meets realism floors |  |
| new_geography | iso6346 | iso6346 | **keep** | pass_floors=True — meets realism floors |  |
| new_geography | mgrs | mgrs | **keep** | pass_floors=True — meets realism floors |  |
| new_geography | plus_code | plus code | **keep** | pass_floors=True — meets realism floors |  |
| new_geography | unlocode | unlocode | **keep** | pass_floors=True — meets realism floors |  |
| new_geography | wkt | wkt | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | abn | abn | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | cpt | cpt | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | ein | ein | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | email_display | email display | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | eu_vat | eu vat | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | hcpcs | hcpcs | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | icd10 | icd10 | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | isrc | isrc | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | loinc | loinc | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | orcid | orcid | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | pan_india | pan india | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | phone_e164 | phone e164 | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | ssn | ssn | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | upc | upc | **keep** | pass_floors=True — meets realism floors |  |
| new_identity | vin | vin | **keep** | pass_floors=True — meets realism floors |  |
| new_representation | cas_number | cas number | **keep** | pass_floors=True — meets realism floors |  |
| new_representation | color_hsl | color hsl | **keep** | pass_floors=True — meets realism floors |  |
| new_representation | inchi | inchi | **keep** | pass_floors=True — meets realism floors |  |
| new_representation | smiles | smiles | **keep** | pass_floors=True — meets realism floors |  |
| new_technology | aws_arn | aws arn | **keep** | pass_floors=True — meets realism floors |  |
| new_technology | cidr | cidr | **keep** | pass_floors=True — meets realism floors |  |
| new_technology | data_uri | data uri | **keep** | pass_floors=True — meets realism floors |  |
| new_technology | docker_ref | docker ref | **keep** | pass_floors=True — meets realism floors |  |
| new_technology | git_sha | git sha | **keep** | pass_floors=True — meets realism floors |  |
| new_technology | jwt | jwt | **keep** | pass_floors=True — meets realism floors |  |
| new_technology | s3_uri | s3 uri | **keep** | pass_floors=True — meets realism floors |  |
| new_technology | snowflake_id | snowflake id | **keep** | pass_floors=True — meets realism floors |  |
| new_technology | tsid | tsid | **keep** | pass_floors=True — meets realism floors |  |
| new_technology | ulid | ulid | **keep** | pass_floors=True — meets realism floors |  |
| new_technology | urn | urn | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | age | number | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | company | entity name | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | date_of_birth | iso date | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | email | email | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | first_name | first name | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | full_name | name | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | gender | gender | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | height_cm | height | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | job_title | occupation | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | last_name | last name | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | phone | telephone | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | salary | number | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | ssn | ssn | **keep** | pass_floors=True — meets realism floors |  |
| people_directory | weight_kg | weight | **keep** | pass_floors=True — meets realism floors |  |
| representation_coverage | decimal_number_comma | decimal number comma | **keep** | pass_floors=True — meets realism floors |  |
| representation_coverage | file_size | file size | **keep** | pass_floors=True — meets realism floors |  |
| representation_coverage | integer_number | integer number | **keep** | pass_floors=True — meets realism floors |  |
| representation_coverage | scientific_notation | scientific notation | **keep** | pass_floors=True — meets realism floors |  |
| scientific_measurements | experiment_id | id | **keep** | pass_floors=True — meets realism floors |  |
| scientific_measurements | formula | code | **keep** | pass_floors=True — meets realism floors |  |
| scientific_measurements | latitude | latitude | **keep** | pass_floors=True — meets realism floors |  |
| scientific_measurements | longitude | longitude | **keep** | pass_floors=True — meets realism floors |  |
| scientific_measurements | measurement_unit | measurement unit | **keep** | pass_floors=True — meets realism floors |  |
| scientific_measurements | percentage | percentage | **keep** | pass_floors=True — meets realism floors |  |
| scientific_measurements | ph_value | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| scientific_measurements | pressure_atm | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| scientific_measurements | temperature_celsius | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| scientific_measurements | timestamp | timestamp | **keep** | pass_floors=True — meets realism floors |  |
| scientific_measurements | value | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| server_logs_json | bytes_sent | number | **keep** | pass_floors=True — meets realism floors |  |
| server_logs_json | client_ip | ip_v4 | **keep** | pass_floors=True — meets realism floors |  |
| server_logs_json | method | http method | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| server_logs_json | path | route | **keep** | pass_floors=True — meets realism floors |  |
| server_logs_json | request_url | url | **keep** | pass_floors=True — meets realism floors |  |
| server_logs_json | response_time_ms | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| server_logs_json | status_code | http status code | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| server_logs_json | timestamp | iso timestamp milliseconds | **keep** | pass_floors=True — meets realism floors |  |
| server_logs_json | user_agent | user agent | **keep** | pass_floors=True — meets realism floors |  |
| sports_events | attendance | number | **keep** | pass_floors=True — meets realism floors |  |
| sports_events | country | country | **keep** | pass_floors=True — meets realism floors |  |
| sports_events | duration_minutes | number | **keep** | pass_floors=True — meets realism floors |  |
| sports_events | event_date | iso date | **keep** | pass_floors=True — meets realism floors |  |
| sports_events | event_id | code | **keep** | pass_floors=True — meets realism floors |  |
| sports_events | is_broadcast | boolean | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| sports_events | sport | category | **keep** | pass_floors=True — meets realism floors |  |
| sports_events | start_time | time 24h | **keep** | pass_floors=True — meets realism floors |  |
| sports_events | status | status | **keep** | pass_floors=True — meets realism floors |  |
| sports_events | ticket_price | price | **keep** | pass_floors=True — meets realism floors |  |
| sports_events | venue | entity name | **keep** | pass_floors=True — meets realism floors |  |
| sports_events | viewer_rating | percentage | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | api_key | code | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | ip_address | ip_v4 | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | language | language | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | log_timestamp | timestamp | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | mac_address | mac address | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | os | operating system | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | port | port | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | request_url | url | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | server_hostname | hostname | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | user_agent | user agent | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | uuid | uuid | **keep** | pass_floors=True — meets realism floors |  |
| tech_systems | version | version | **keep** | pass_floors=True — meets realism floors |  |
| technology_coverage | calver | calver | **keep** | pass_floors=True — meets realism floors |  |
| technology_coverage | ip_v4_with_port | ip_v4 with port | **keep** | pass_floors=True — meets realism floors |  |
| technology_coverage | ip_v6 | ip_v6 | **keep** | pass_floors=True — meets realism floors |  |
| technology_coverage | username | username | **keep** | pass_floors=True — meets realism floors |  |
| titanic | Age | number | **keep** | pass_floors=True — meets realism floors |  |
| titanic | Cabin | code | **keep** | high-null semi-structured / optional field; real-world sparsity |  |
| titanic | Embarked | category | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| titanic | Fare | price | **keep** | pass_floors=True — meets realism floors |  |
| titanic | Name | name | **keep** | pass_floors=True — meets realism floors |  |
| titanic | Parch | number | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| titanic | PassengerId | id | **keep** | pass_floors=True — meets realism floors |  |
| titanic | Pclass | class | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| titanic | Sex | gender | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| titanic | SibSp | number | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| titanic | Survived | boolean | **keep** | low-cardinality categorical signature; floor too strict for enum family |  |
| titanic | Ticket | code | **keep** | pass_floors=True — meets realism floors |  |
| us_states | Abbreviation | code | **keep** | pass_floors=True — meets realism floors |  |
| us_states | State | state | **keep** | pass_floors=True — meets realism floors |  |
| weather_stations_json | humidity_pct | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| weather_stations_json | location.city | city | **keep** | high-null semi-structured / optional field; real-world sparsity |  |
| weather_stations_json | location.country | country code | **keep** | high-null semi-structured / optional field; real-world sparsity |  |
| weather_stations_json | location.latitude | latitude | **keep** | high-null semi-structured / optional field; real-world sparsity |  |
| weather_stations_json | location.longitude | longitude | **keep** | high-null semi-structured / optional field; real-world sparsity |  |
| weather_stations_json | observation_date | iso date | **keep** | pass_floors=True — meets realism floors |  |
| weather_stations_json | precipitation_mm | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| weather_stations_json | station_name | entity name | **keep** | pass_floors=True — meets realism floors |  |
| weather_stations_json | temperature_c | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| weather_stations_json | wind_speed_kmh | decimal number | **keep** | pass_floors=True — meets realism floors |  |
| world_cities | country | country | **keep** | pass_floors=True — meets realism floors |  |
| world_cities | geonameid | id | **keep** | pass_floors=True — meets realism floors |  |
| world_cities | name | city | **keep** | pass_floors=True — meets realism floors |  |
| world_cities | subcountry | region | **keep** | pass_floors=True — meets realism floors |  |
