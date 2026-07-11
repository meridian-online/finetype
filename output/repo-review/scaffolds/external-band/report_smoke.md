# External-data advisory band — report

**Status:** ADVISORY (never blocking). Read the candidate-vs-baseline delta, not the absolute — labels overlap gold, so the absolute is common-mode.
**Rotation:** rotate=1 seed=3 tables=1
**Binary:** `target/release/finetype`

## Headline: 4/5 = 0.800 (8 unlabelled emissions triaged)

## Per-type (adjudicated columns only)

| label | correct | total | recall |
|---|---|---|---|
| datetime.component.year | 1 | 1 | 1.000 |
| datetime.timestamp.sql_standard | 1 | 1 | 1.000 |
| representation.numeric.decimal_number | 2 | 2 | 1.000 |
| representation.text.word | 0 | 1 | 0.000 |

## Misses (adjudicated gold != predicted)

| table | column | gold | predicted |
|---|---|---|---|
| nyc_payroll_sample.csv | leave_status_as_of_june_30 | representation.text.word | representation.boolean.terms |

## Unlabelled emissions (triage — NOT in the headline)

These full-table columns have no adjudicated label yet. This is the candidate-expansion queue: an over-emission here (e.g. a ticker read as an NPI) is the failure class this band exists to surface. Adjudicate + sign off before any of these counts toward a headline.

| table | column | predicted |
|---|---|---|
| nyc_payroll_sample.csv | agency_name | geography.location.region |
| nyc_payroll_sample.csv | base_salary | representation.numeric.decimal_number |
| nyc_payroll_sample.csv | pay_basis | representation.text.plain_text |
| nyc_payroll_sample.csv | regular_gross_paid | representation.numeric.decimal_number |
| nyc_payroll_sample.csv | title_description | unknown |
| nyc_payroll_sample.csv | total_ot_paid | representation.numeric.decimal_number |
| nyc_payroll_sample.csv | total_other_pay | representation.numeric.decimal_number |
| nyc_payroll_sample.csv | work_location_borough | geography.location.region |
