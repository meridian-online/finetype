# External-data advisory band — report

**Status:** ADVISORY (never blocking). Read the candidate-vs-baseline delta, not the absolute — held labels overlap gold, so the absolute is common-mode across candidates. No headline here overrides a blocking corpus-honest NO-GO.
**Binary:** `target/release/finetype`
**Rotation:** rotate=all seed=0 tables=15 (chicago_crimes_sample.csv, compound_codelist.csv, gleif_entities.csv, icd10_codes.csv, majestic_million_top20k.csv, naics_codes.csv, nyc_dob_permits_sample.csv, nyc_payroll_sample.csv, openflights_airports.csv, ourairports_airports.csv, seattle_checkouts_sample.csv, sec_edgar_companies.csv, sf_businesses_sample.csv, uk_price_paid_sample.csv, usgs_earthquakes_202605.csv)
**Label source:** live-derived from `eval/gold/gold_corpus.tsv` (gold rows pointing at `eval/datasets/gold_external/`).

## Headline: 90/145 = 0.621 (52 unlabelled emissions triaged)

### Per-table

| table | correct | scored | headline |
|---|---|---|---|
| chicago_crimes_sample.csv | 10 | 15 | 0.667 |
| compound_codelist.csv | 1 | 3 | 0.333 |
| gleif_entities.csv | 9 | 13 | 0.692 |
| icd10_codes.csv | 2 | 2 | 1.000 |
| majestic_million_top20k.csv | 5 | 8 | 0.625 |
| naics_codes.csv | 1 | 2 | 0.500 |
| nyc_dob_permits_sample.csv | 5 | 16 | 0.312 |
| nyc_payroll_sample.csv | 5 | 13 | 0.385 |
| openflights_airports.csv | 11 | 12 | 0.917 |
| ourairports_airports.csv | 10 | 14 | 0.714 |
| seattle_checkouts_sample.csv | 8 | 11 | 0.727 |
| sec_edgar_companies.csv | 2 | 3 | 0.667 |
| sf_businesses_sample.csv | 9 | 12 | 0.750 |
| uk_price_paid_sample.csv | 5 | 10 | 0.500 |
| usgs_earthquakes_202605.csv | 7 | 11 | 0.636 |

### Tier mix of scored labels (gold-overlap disclosure)

| tier (labeller) | count |
|---|---|
| llm-adjudicated-2panel | 50 |
| lens-consensus | 50 |
| llm-3panel-blind | 21 |
| author-adjudicated | 15 |
| agent-readjudicated | 7 |
| author | 1 |
| llm-3panel-blind+adversarial | 1 |

## Per-type (adjudicated columns only)

| label | correct | total | recall |
|---|---|---|---|
| datetime.component.year | 3 | 4 | 0.750 |
| datetime.date.iso | 3 | 3 | 1.000 |
| datetime.date.mdy_slash | 0 | 4 | 0.000 |
| datetime.offset.iana | 1 | 1 | 1.000 |
| datetime.timestamp.iso_8601_milliseconds | 2 | 2 | 1.000 |
| datetime.timestamp.iso_milliseconds | 5 | 5 | 1.000 |
| datetime.timestamp.sql_standard | 1 | 2 | 0.500 |
| finance.currency.amount | 0 | 5 | 0.000 |
| finance.securities.lei | 1 | 1 | 1.000 |
| geography.address.postal_code | 3 | 3 | 1.000 |
| geography.coordinate.latitude | 5 | 5 | 1.000 |
| geography.coordinate.longitude | 4 | 5 | 0.800 |
| geography.location.city | 5 | 5 | 1.000 |
| geography.location.country | 1 | 1 | 1.000 |
| geography.location.country_code | 3 | 3 | 1.000 |
| geography.location.region | 3 | 5 | 0.600 |
| geography.location.state_code | 2 | 2 | 1.000 |
| geography.transportation.iata_code | 2 | 2 | 1.000 |
| geography.transportation.icao_code | 1 | 1 | 1.000 |
| identity.commerce.isbn | 1 | 1 | 1.000 |
| identity.industry.naics | 1 | 1 | 1.000 |
| identity.medical.icd10 | 1 | 2 | 0.500 |
| identity.person.full_name | 1 | 1 | 1.000 |
| representation.boolean.terms | 2 | 2 | 1.000 |
| representation.identifier.alphanumeric_id | 3 | 7 | 0.429 |
| representation.identifier.numeric_code | 2 | 4 | 0.500 |
| representation.numeric.decimal_number | 4 | 5 | 0.800 |
| representation.numeric.integer_number | 10 | 13 | 0.769 |
| representation.text.entity_name | 2 | 6 | 0.333 |
| representation.text.plain_text | 2 | 11 | 0.182 |
| representation.text.word | 13 | 27 | 0.481 |
| technology.internet.hostname | 2 | 2 | 1.000 |
| technology.internet.top_level_domain | 0 | 2 | 0.000 |
| technology.internet.url | 1 | 2 | 0.500 |

## Misses (adjudicated gold != predicted)

| table | column | gold | predicted | tier |
|---|---|---|---|---|
| chicago_crimes_sample.csv | block | representation.text.plain_text | representation.identifier.alphanumeric_id | llm-adjudicated-2panel |
| chicago_crimes_sample.csv | fbi_code | representation.identifier.alphanumeric_id | representation.identifier.numeric_code | author-adjudicated |
| chicago_crimes_sample.csv | iucr | representation.identifier.numeric_code | datetime.date.compact_dmy | llm-adjudicated-2panel |
| chicago_crimes_sample.csv | location_description | representation.text.word | representation.text.entity_name | llm-adjudicated-2panel |
| chicago_crimes_sample.csv | x_coordinate | representation.numeric.integer_number | unknown | lens-consensus |
| compound_codelist.csv | code | identity.medical.icd10 | geography.address.postal_code | llm-3panel-blind |
| compound_codelist.csv | title | representation.text.plain_text | representation.text.entity_name | llm-3panel-blind |
| gleif_entities.csv | category | representation.text.word | geography.location.region | llm-3panel-blind |
| gleif_entities.csv | entity_status | representation.text.word | representation.boolean.terms | llm-3panel-blind |
| gleif_entities.csv | legal_form | representation.identifier.alphanumeric_id | geography.address.postal_code | llm-3panel-blind |
| gleif_entities.csv | name | representation.text.entity_name | geography.location.region | llm-3panel-blind |
| majestic_million_top20k.csv | GlobalRank | representation.numeric.integer_number | representation.identifier.increment | lens-consensus |
| majestic_million_top20k.csv | IDN_TLD | technology.internet.top_level_domain | geography.location.continent | lens-consensus |
| majestic_million_top20k.csv | TLD | technology.internet.top_level_domain | geography.location.continent | lens-consensus |
| naics_codes.csv | title | representation.text.plain_text | representation.text.entity_name | llm-3panel-blind |
| nyc_dob_permits_sample.csv | expiration_date | datetime.date.mdy_slash | datetime.date.iso | llm-adjudicated-2panel |
| nyc_dob_permits_sample.csv | filing_date | datetime.date.mdy_slash | datetime.date.iso | llm-adjudicated-2panel |
| nyc_dob_permits_sample.csv | gis_longitude | geography.coordinate.longitude | geography.coordinate.latitude | llm-adjudicated-2panel |
| nyc_dob_permits_sample.csv | gis_nta_name | representation.text.plain_text | geography.location.region | llm-adjudicated-2panel |
| nyc_dob_permits_sample.csv | issuance_date | datetime.date.mdy_slash | datetime.date.iso | llm-adjudicated-2panel |
| nyc_dob_permits_sample.csv | job__ | representation.numeric.integer_number | representation.text.word | lens-consensus |
| nyc_dob_permits_sample.csv | job_start_date | datetime.date.mdy_slash | datetime.date.iso | llm-adjudicated-2panel |
| nyc_dob_permits_sample.csv | job_type | representation.text.word | representation.identifier.alphanumeric_id | llm-adjudicated-2panel |
| nyc_dob_permits_sample.csv | permit_type | representation.text.word | geography.location.country_code | lens-consensus |
| nyc_dob_permits_sample.csv | street_name | representation.text.plain_text | geography.address.street_address | lens-consensus |
| nyc_dob_permits_sample.csv | work_type | representation.text.word | geography.location.state_code | lens-consensus |
| nyc_payroll_sample.csv | agency_name | representation.text.entity_name | geography.location.region | agent-readjudicated |
| nyc_payroll_sample.csv | base_salary | finance.currency.amount | representation.numeric.decimal_number | author-adjudicated |
| nyc_payroll_sample.csv | leave_status_as_of_june_30 | representation.text.word | representation.boolean.terms | lens-consensus |
| nyc_payroll_sample.csv | pay_basis | representation.text.word | representation.text.plain_text | llm-adjudicated-2panel |
| nyc_payroll_sample.csv | regular_gross_paid | finance.currency.amount | representation.numeric.decimal_number | author-adjudicated |
| nyc_payroll_sample.csv | title_description | representation.text.plain_text | unknown | author-adjudicated |
| nyc_payroll_sample.csv | total_ot_paid | finance.currency.amount | representation.numeric.decimal_number | author-adjudicated |
| nyc_payroll_sample.csv | total_other_pay | finance.currency.amount | representation.numeric.decimal_number | author-adjudicated |
| openflights_airports.csv | name | representation.text.entity_name | unknown | lens-consensus |
| ourairports_airports.csv | continent | representation.text.word | geography.location.continent | llm-adjudicated-2panel |
| ourairports_airports.csv | home_link | technology.internet.url | technology.code.qualified_name | lens-consensus |
| ourairports_airports.csv | iso_region | geography.location.region | unknown | llm-adjudicated-2panel |
| ourairports_airports.csv | name | representation.text.entity_name | geography.address.full_address | lens-consensus |
| seattle_checkouts_sample.csv | checkouttype | representation.text.word | geography.location.region | llm-adjudicated-2panel |
| seattle_checkouts_sample.csv | materialtype | representation.text.word | unknown | llm-adjudicated-2panel |
| seattle_checkouts_sample.csv | publicationyear | datetime.component.year | unknown | author-adjudicated |
| sec_edgar_companies.csv | ticker | representation.text.word | geography.location.state_code | llm-3panel-blind+adversarial |
| sf_businesses_sample.csv | naic_code | representation.identifier.numeric_code | representation.numeric.integer_number | llm-adjudicated-2panel |
| sf_businesses_sample.csv | naic_code_description | representation.text.word | representation.text.entity_name | llm-adjudicated-2panel |
| sf_businesses_sample.csv | neighborhoods_analysis_boundaries | representation.text.plain_text | geography.location.city | author-adjudicated |
| uk_price_paid_sample.csv | county | geography.location.region | unknown | llm-adjudicated-2panel |
| uk_price_paid_sample.csv | date_of_transfer | datetime.timestamp.sql_standard | unknown | lens-consensus |
| uk_price_paid_sample.csv | price | finance.currency.amount | representation.numeric.integer_number | author-adjudicated |
| uk_price_paid_sample.csv | street | representation.text.plain_text | geography.address.street_address | lens-consensus |
| uk_price_paid_sample.csv | transaction_id | representation.identifier.alphanumeric_id | container.object.json | llm-adjudicated-2panel |
| usgs_earthquakes_202605.csv | id | representation.identifier.alphanumeric_id | geography.coordinate.geohash | lens-consensus |
| usgs_earthquakes_202605.csv | mag | representation.numeric.decimal_number | unknown | llm-adjudicated-2panel |
| usgs_earthquakes_202605.csv | net | representation.text.word | geography.location.region | llm-adjudicated-2panel |
| usgs_earthquakes_202605.csv | place | representation.text.plain_text | geography.address.full_address | llm-adjudicated-2panel |

## Unlabelled emissions (triage — NOT in the headline)

Profiled columns with no adjudicated label yet. This is the candidate-expansion queue: an over-emission here (e.g. a ticker read as a state code) is the failure class this band exists to surface. Adjudicate + assign a truth tier before any of these counts toward a headline.

| table | column | predicted |
|---|---|---|
| chicago_crimes_sample.csv | beat | representation.identifier.numeric_code |
| chicago_crimes_sample.csv | community_area | representation.numeric.integer_number |
| chicago_crimes_sample.csv | description | representation.text.entity_name |
| chicago_crimes_sample.csv | district | representation.numeric.integer_number |
| chicago_crimes_sample.csv | domestic | representation.boolean.terms |
| chicago_crimes_sample.csv | id | representation.numeric.integer_number |
| chicago_crimes_sample.csv | location | geography.coordinate.coordinates |
| majestic_million_top20k.csv | PrevGlobalRank | representation.numeric.integer_number |
| majestic_million_top20k.csv | PrevRefIPs | representation.numeric.integer_number |
| majestic_million_top20k.csv | PrevRefSubNets | representation.numeric.integer_number |
| majestic_million_top20k.csv | PrevTldRank | representation.numeric.integer_number |
| nyc_dob_permits_sample.csv | bldg_type | datetime.date.compact_dmy |
| nyc_dob_permits_sample.csv | block | representation.numeric.integer_number |
| nyc_dob_permits_sample.csv | community_board | representation.numeric.integer_number |
| nyc_dob_permits_sample.csv | gis_census_tract | representation.numeric.integer_number |
| nyc_dob_permits_sample.csv | gis_council_district | representation.numeric.integer_number |
| nyc_dob_permits_sample.csv | house__ | representation.numeric.integer_number |
| nyc_dob_permits_sample.csv | lot | representation.numeric.integer_number |
| nyc_dob_permits_sample.csv | permit_sequence__ | representation.numeric.integer_number |
| nyc_dob_permits_sample.csv | permit_subtype | geography.location.region |
| nyc_dob_permits_sample.csv | site_fill | representation.text.word |
| openflights_airports.csv | airport_id | representation.numeric.integer_number |
| openflights_airports.csv | source | representation.text.entity_name |
| ourairports_airports.csv | icao_code | geography.transportation.icao_code |
| ourairports_airports.csv | id | representation.numeric.integer_number |
| ourairports_airports.csv | ident | geography.address.postal_code |
| ourairports_airports.csv | keywords | representation.text.entity_name |
| ourairports_airports.csv | local_code | representation.identifier.alphanumeric_id |
| seattle_checkouts_sample.csv | subjects | representation.text.plain_text |
| sf_businesses_sample.csv | administratively_closed | representation.text.plain_text |
| sf_businesses_sample.csv | dba_end_date | datetime.timestamp.iso_milliseconds |
| sf_businesses_sample.csv | location_end_date | datetime.timestamp.iso_milliseconds |
| sf_businesses_sample.csv | parking_tax | representation.boolean.terms |
| sf_businesses_sample.csv | transient_occupancy_tax | representation.boolean.terms |
| sf_businesses_sample.csv | uniqueid | representation.identifier.alphanumeric_id |
| uk_price_paid_sample.csv | locality | geography.location.state_code |
| uk_price_paid_sample.csv | old_new | representation.boolean.initials |
| uk_price_paid_sample.csv | paon | representation.text.entity_name |
| uk_price_paid_sample.csv | ppd_category_type | identity.person.blood_type |
| uk_price_paid_sample.csv | record_status | representation.text.word |
| uk_price_paid_sample.csv | saon | representation.text.entity_name |
| usgs_earthquakes_202605.csv | depthError | representation.numeric.decimal_number |
| usgs_earthquakes_202605.csv | dmin | representation.numeric.decimal_number |
| usgs_earthquakes_202605.csv | gap | representation.numeric.integer_number |
| usgs_earthquakes_202605.csv | horizontalError | representation.numeric.decimal_number |
| usgs_earthquakes_202605.csv | locationSource | geography.location.region |
| usgs_earthquakes_202605.csv | magError | representation.numeric.decimal_number |
| usgs_earthquakes_202605.csv | magNst | representation.numeric.integer_number |
| usgs_earthquakes_202605.csv | magSource | geography.location.region |
| usgs_earthquakes_202605.csv | nst | representation.numeric.integer_number |
| usgs_earthquakes_202605.csv | rms | representation.numeric.decimal_number |
| usgs_earthquakes_202605.csv | type | geography.location.region |
