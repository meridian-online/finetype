# External-data advisory band — report

**Status:** ADVISORY (never blocking). Read the candidate-vs-baseline delta, not the absolute — held labels overlap gold, so the absolute is common-mode across candidates. No headline here overrides a blocking corpus-honest NO-GO.
**Binary:** `target/release/finetype`
**Rotation:** rotate=all seed=0 tables=3 (gleif_entities.csv, nyc_dob_permits_sample.csv, sec_edgar_companies.csv)
**Label source:** live-derived from `eval/gold/gold_corpus.tsv` (gold rows pointing at `eval/datasets/gold_external/`).

## Headline: 16/32 = 0.500 (10 unlabelled emissions triaged)

### Per-table

| table | correct | scored | headline |
|---|---|---|---|
| gleif_entities.csv | 9 | 13 | 0.692 |
| nyc_dob_permits_sample.csv | 5 | 16 | 0.312 |
| sec_edgar_companies.csv | 2 | 3 | 0.667 |

### Tier mix of scored labels (gold-overlap disclosure)

| tier (labeller) | count |
|---|---|
| llm-3panel-blind | 14 |
| llm-adjudicated-2panel | 8 |
| lens-consensus | 7 |
| author | 1 |
| author-adjudicated | 1 |
| llm-3panel-blind+adversarial | 1 |

## Per-type (adjudicated columns only)

| label | correct | total | recall |
|---|---|---|---|
| datetime.date.iso | 3 | 3 | 1.000 |
| datetime.date.mdy_slash | 0 | 4 | 0.000 |
| finance.securities.lei | 1 | 1 | 1.000 |
| geography.address.postal_code | 1 | 1 | 1.000 |
| geography.coordinate.latitude | 1 | 1 | 1.000 |
| geography.coordinate.longitude | 0 | 1 | 0.000 |
| geography.location.city | 1 | 1 | 1.000 |
| geography.location.country_code | 2 | 2 | 1.000 |
| geography.location.region | 1 | 1 | 1.000 |
| geography.location.state_code | 1 | 1 | 1.000 |
| representation.identifier.alphanumeric_id | 0 | 1 | 0.000 |
| representation.identifier.numeric_code | 1 | 1 | 1.000 |
| representation.numeric.integer_number | 0 | 1 | 0.000 |
| representation.text.entity_name | 1 | 2 | 0.500 |
| representation.text.plain_text | 0 | 2 | 0.000 |
| representation.text.word | 3 | 9 | 0.333 |

## Misses (adjudicated gold != predicted)

| table | column | gold | predicted | tier |
|---|---|---|---|---|
| gleif_entities.csv | category | representation.text.word | geography.location.region | llm-3panel-blind |
| gleif_entities.csv | entity_status | representation.text.word | representation.boolean.terms | llm-3panel-blind |
| gleif_entities.csv | legal_form | representation.identifier.alphanumeric_id | geography.address.postal_code | llm-3panel-blind |
| gleif_entities.csv | name | representation.text.entity_name | geography.location.region | llm-3panel-blind |
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
| sec_edgar_companies.csv | ticker | representation.text.word | geography.location.state_code | llm-3panel-blind+adversarial |

## Unlabelled emissions (triage — NOT in the headline)

Profiled columns with no adjudicated label yet. This is the candidate-expansion queue: an over-emission here (e.g. a ticker read as a state code) is the failure class this band exists to surface. Adjudicate + assign a truth tier before any of these counts toward a headline.

| table | column | predicted |
|---|---|---|
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
