# External band — full-column re-check (2026-07-11)

Objective test: over the WHOLE column (not a 12-row sample), what fraction of values pass the model-predicted label's pattern vs the gold label's pattern. This corrects the initial adjudication, which used first-12-row samples that were unrepresentative for the sorted NYC date columns.

| table | column | model pred | pred pass% | gold | gold pass% | verdict |
|---|---|---|---|---|---|---|
| sec | ticker | state_code | 14% | word | 100% | MODEL ERROR |
| gleif | name | region | len-only | entity_name | len-only | MODEL ERROR |
| gleif | category | region | len-only | word | 100% | MODEL ERROR |
| gleif | legal_form | postal_code | 91% | alphanumeric_id | 6% | MODEL ERROR (gold imperfect) |
| gleif | entity_status | terms | len-only | word | 100% | GOLD ERROR (model better) |
| nyc | street_name | street_address | len-only | plain_text | len-only | MODEL ERROR |
| nyc | job__ | word | 100% | integer_number | 100% | MODEL ERROR |
| nyc | job_type | alphanumeric_id | 79% | word | 100% | MODEL ERROR |
| nyc | work_type | state_code | 100% | word | 100% | MODEL ERROR |
| nyc | permit_type | country_code | 100% | word | 100% | MODEL ERROR |
| nyc | gis_longitude | latitude | 100% | longitude | 100% | MODEL ERROR |
| nyc | gis_nta_name | region | len-only | plain_text | len-only | MODEL ERROR |
| nyc | filing_date | iso | 83% | mdy_slash | 17% | GOLD ERROR (model better) |
| nyc | issuance_date | iso | 83% | mdy_slash | 17% | GOLD ERROR (model better) |
| nyc | expiration_date | iso | 83% | mdy_slash | 17% | GOLD ERROR (model better) |
| nyc | job_start_date | iso | 83% | mdy_slash | 17% | GOLD ERROR (model better) |

**Result: 11 genuine model errors, 5 gold-label errors** (4 dates where the column is ~83% iso so the model's iso validates the majority and gold's mdy_slash rejects it; + entity_status). 'len-only' = the label is length/shape-only with no discriminating pattern, so validation can't referee — semantics decide (all such cases are clear: org names/neighbourhood names are not geography regions).
